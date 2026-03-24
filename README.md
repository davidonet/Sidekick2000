# Sidekick2000

A macOS desktop app that records meetings, transcribes them, and produces structured notes — with action items pushed directly as GitHub issues and notes committed to a git repository.

Built with [Tauri](https://tauri.app) + Svelte 5 on the frontend, Rust on the backend.

---

## What it does

1. **Records** two audio streams in parallel — local mic and remote source (system audio loopback) — with live dual VU meters before and during recording
2. **Transcribes** both streams simultaneously using [Groq Whisper](https://groq.com) (fast, multilingual)
3. **Merges** the two transcripts into a single time-sorted conversation, each segment already labelled with the correct speaker name (no diarization needed)
4. **Summarizes** with Claude Sonnet (Anthropic) or any [Together.ai](https://www.together.ai) chat model, using a context you define (meeting type, participants, domain vocabulary)
5. **Exports** structured Markdown notes (`YYYY-MM-DD_HHmm_Context.md`) to a configurable folder
6. **Commits** the notes to a git repository automatically _(optional)_
7. **Creates GitHub issues** for every action item extracted from the notes _(optional)_

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

- macOS (Apple Silicon or Intel)
- [Groq API key](https://console.groq.com) — for Whisper transcription
- **One of** (for summarization):
  - [Anthropic API key](https://console.anthropic.com) — to use Claude Sonnet
  - [Together.ai API key](https://www.together.ai) — to use open-source models (Llama, etc.)
- [`gh` CLI](https://cli.github.com) installed and authenticated — for GitHub issues _(optional)_
- `git` — for committing notes _(optional)_

### Install dependencies

```bash
npm install
```

### Configure

Launch the app and click the **gear icon** in the top-right corner. All settings are stored in `~/.sidekick2000/settings.json`.

| Tab | What to configure |
|-----|-------------------|
| **API Keys** | Groq key; summarization provider (Claude or Together.ai) and the corresponding API key/model |
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
Record local mic  ─────────────────┐   (OGG/Opus, parallel)
Record remote source ───────────────┤
                                    │
                    ┌───────────────┴───────────────┐
                    ▼                               ▼
         Transcribe local                  Transcribe remote
         (Groq Whisper)                    (Groq Whisper)
                    └───────────────┬───────────────┘
                                    │
                                    ▼
                    Merge — sort by timestamp, speakers already known
                                    │
                                    ▼
                    Summarize  (Claude Sonnet 4.6 or Together.ai — skipped if disabled)
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
  "groq_api_key": "gsk_...",
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
| Transcription | Groq Whisper API (`whisper-large-v3-turbo`), two streams in parallel |
| Summarization | Anthropic Claude Sonnet 4.6 or Together.ai (configurable) |
| Speaker identification | Device-based — each stream has a pre-assigned speaker name |
| Audio capture | CPAL (two simultaneous input streams) |
| GitHub integration | `gh` CLI |
