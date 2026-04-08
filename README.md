# Sidekick2000

A macOS desktop app that records meetings, transcribes them, and produces structured notes — with action items pushed directly as GitHub issues and notes committed to a git repository.

Built with [Tauri](https://tauri.app) + Svelte 5 on the frontend, Rust on the backend.

---

## What it does

1. **Records** two audio streams in parallel — local mic and remote source (system audio via BlackHole) — with live dual VU meters
2. **Transcribes live** both streams in real time using local [whisper.cpp](https://github.com/ggerganov/whisper.cpp) with Metal GPU acceleration — no audio leaves your machine. VAD (Silero) detects speech boundaries and feeds chunks to Whisper as you speak, so the transcript appears during the meeting.
3. **Merges** the two transcripts into a single time-sorted conversation, each segment labelled with the correct speaker name (one stream = one speaker, no diarization needed)
4. **Summarizes** with Claude Sonnet (Anthropic) or any [Together.ai](https://www.together.ai) chat model, using a context you define (meeting type, participants, domain vocabulary)
5. **Exports** structured Markdown notes (`YYYY-MM-DD_HHmm_Context.md`) to a configurable folder
6. **Commits** the notes to a git repository automatically _(optional)_
7. **Creates GitHub issues** for every action item extracted from the notes _(optional)_

> **Privacy:** All audio is processed locally on-device. Only the final text transcript is sent to the cloud for summarization (Anthropic or Together.ai). A [Groq Whisper](https://groq.com) cloud fallback is available in settings if preferred.

---

## Output format

Each meeting produces two files:

```
Meetings/
  2026-03-20_1430_welqin.md          ← structured notes
  2026-03-20_1430_welqin_transcript.md ← raw transcript with named speakers
```

The notes follow a consistent structure:

```markdown
## Participants
## Summary
## Key Discussion Points
## Decisions Made
## Action Items
- [ ] **David**: Review the API design document
- [ ] **Yannick**: Set up CI pipeline for staging
```

Action items are automatically created as GitHub issues with the `meeting-action` label.

---

## Setup

### Requirements

- macOS Apple Silicon (M1 or later — Metal GPU required for local Whisper)
- [BlackHole 2ch](https://existential.audio/blackhole/) — virtual audio driver for capturing system audio (see [setup guide](docs/DUAL_STREAM.md))
- `cmake` — required to build whisper.cpp (`brew install cmake`)
- **One of** (for summarization):
  - [Anthropic API key](https://console.anthropic.com) — to use Claude Sonnet
  - [Together.ai API key](https://www.together.ai) — to use open-source models (Llama, etc.)
- [`gh` CLI](https://cli.github.com) installed and authenticated — for GitHub issues _(optional)_
- `git` — for committing notes _(optional)_
- [Groq API key](https://console.groq.com) — only if using cloud transcription fallback _(optional)_

> The Whisper model (`large-v3-turbo`, quantized q5_0, ~550 MB) is downloaded automatically on first launch to `~/.sidekick2000/models/`.

### Install dependencies

```bash
npm install
```

### Configure

Launch the app and click the **gear icon** in the top-right corner. All settings are stored in `~/.sidekick2000/settings.json`.

| Tab | What to configure |
|-----|-------------------|
| **API Keys** | Transcription mode (Local Whisper or Groq); summarization provider (Claude or Together.ai) and the corresponding API key/model |
| **Devices** | Local mic device + your speaker name; remote source device + remote speaker name |
| **Repository** | Working folder (git root), meetings subfolder, GitHub repo (`owner/repo`), default language, pipeline step toggles |
| **Contexts** | Meeting context templates — instructions that shape how the AI summarizes each meeting type |
| **Speakers** | Default meeting attendees pre-loaded at startup (for AI context) |

> API keys set in Settings take priority over environment variables. You can still use a `.env` file as fallback.

### Dev mode

```bash
npm run tauri dev
```

### Build

```bash
npm run tauri build
```

---

## Contexts

Contexts are the core of Sidekick2000's flexibility. Each context is a Markdown document that gives Claude background knowledge about a meeting type: who the participants are, domain vocabulary, and how to structure the notes.

Examples of contexts you might create:

- **General** — neutral instructions, works for any meeting
- **Product review** — focus on decisions, feature requests, backlog items
- **Client call** — highlight commitments, risks, next steps
- **Training session** — track exercises, Q&A, shortcuts mentioned

Contexts are managed entirely in the Settings UI (no external files needed).

---

## Pipeline

```
Record local mic  ─────────────────┐   (parallel, ring buffers)
Record remote source ───────────────┤
                                    │
                    ┌───────────────┴───────────────┐
                    ▼                               ▼
           Worker thread                   Worker thread
           (drain every 200ms)             (drain every 200ms)
                    │                               │
                    ▼                               ▼
           VAD (Silero)                    VAD (Silero)
           silence ≥ 300ms → flush         silence ≥ 300ms → flush
                    │                               │
                    ▼                               ▼
           Whisper (Metal)                 Whisper (Metal)
           whisper.cpp local               whisper.cpp local
                    │                               │
                    ▼                               ▼
           emit "live-segment"             emit "live-segment"
           → frontend display              → frontend display
                    └───────────────┬───────────────┘
                                    │
                                    ▼
                    Merge — sort by timestamp, speakers already known
                                    │
                                    ▼
                    Summarize  (Claude Sonnet or Together.ai — skipped if disabled)
                                    │
                                    ▼
                    Export  YYYY-MM-DD_HHmm_Context.md
                                    │
                                    ▼
                    Git commit  (if enabled and working folder configured)
                                    │
                                    ▼
                    Create GitHub issues  (if enabled and repo configured)
```

---

## Settings file

`~/.sidekick2000/settings.json` — created automatically on first save.

```json
{
  "transcription_mode": "LocalWhisper",
  "groq_api_key": "",
  "anthropic_api_key": "sk-ant-...",
  "together_ai_api_key": "",
  "summarization_provider": "claude",
  "together_ai_model": "meta-llama/Llama-3.3-70B-Instruct-Turbo",
  "default_input_device": "MacBook Pro Microphone",
  "local_speaker_name": "David",
  "remote_device": "BlackHole 2ch",
  "remote_speaker_name": "Remote",
  "working_folder": "/Users/you/my-repo",
  "github_repo": "owner/repo",
  "meetings_subfolder": "Meetings",
  "default_language": "fr",
  "enable_summary": true,
  "enable_git_commit": true,
  "enable_github_issues": true,
  "default_speakers": [
    { "name": "Alice", "organization": "Acme" }
  ],
  "contexts": [
    {
      "id": "general",
      "label": "General",
      "content": "Be factual. Group by theme. Use professional tone."
    }
  ]
}
```

---

## Tech stack

| Layer | Technology |
|-------|-----------|
| UI | Svelte 5, Tailwind CSS 4 |
| Desktop shell | Tauri 2 |
| Backend | Rust (async with Tokio) |
| Transcription | Local [whisper.cpp](https://github.com/ggerganov/whisper.cpp) via `whisper-rs` (`large-v3-turbo` q5_0, Metal GPU). Groq Whisper API as optional cloud fallback. |
| VAD | [Silero VAD](https://github.com/snakers4/silero-vad) via `voice_activity_detector` (ONNX) — detects speech/silence for live chunking |
| Summarization | Anthropic Claude Sonnet or Together.ai (configurable) |
| Speaker identification | Device-based — each stream has a pre-assigned speaker name |
| Audio capture | CPAL (two simultaneous input streams, ring buffers, shared t=0 origin) |
| GitHub integration | `gh` CLI |
