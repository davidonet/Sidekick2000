<script lang="ts">
  import { appState } from "../lib/state.svelte";
  import {
    startRecording,
    stopRecording,
    getAudioLevel,
    getElapsed,
    runPipeline,
    loadContextFile,
    onPipelineProgress,
  } from "../lib/api";
  import type { PipelineConfig } from "../lib/types";
  import AudioMeter from "./AudioMeter.svelte";

  let pollingId: ReturnType<typeof setInterval> | null = null;

  async function handleRecord() {
    if (appState.phase === "recording") {
      // Stop recording
      try {
        const [oggPath, wavPath] = await stopRecording();
        appState.oggPath = oggPath;
        appState.wavPath = wavPath;
        if (pollingId) {
          clearInterval(pollingId);
          pollingId = null;
        }
        appState.phase = "processing";
        await startPipeline();
      } catch (e: any) {
        appState.errorMessage = e.toString();
        appState.phase = "error";
      }
    } else {
      // Start recording
      try {
        await startRecording();
        appState.phase = "recording";
        appState.elapsedSecs = 0;
        appState.audioLevel = 0;

        // Poll for audio level and elapsed time
        pollingId = setInterval(async () => {
          try {
            const [level, elapsed] = await Promise.all([
              getAudioLevel(),
              getElapsed(),
            ]);
            appState.audioLevel = level;
            appState.elapsedSecs = elapsed;
          } catch {
            // ignore polling errors
          }
        }, 100);
      } catch (e: any) {
        appState.errorMessage = e.toString();
        appState.phase = "error";
      }
    }
  }

  async function startPipeline() {
    // Set up progress listener
    const unlisten = await onPipelineProgress((step, progress) => {
      appState.pipelineStep = step;
      appState.pipelineProgress = progress;
    });

    try {
      // Load context content
      let contextContent = "";
      if (appState.contextFile === "custom") {
        contextContent = appState.customContext;
      } else {
        contextContent = await loadContextFile(appState.contextFile);
      }

      // Map language code to language name for the prompt
      const langMap: Record<string, string> = {
        fr: "French",
        en: "English",
        es: "Spanish",
        de: "German",
        it: "Italian",
        pt: "Portuguese",
      };

      const config: PipelineConfig = {
        context: appState.contextFile === "custom" ? "custom" : appState.contextFile.replace(".md", ""),
        context_content: contextContent,
        speakers: appState.enabledSpeakers.map((s) => ({
          name: s.name,
          organization: s.organization,
        })),
        language_code: appState.language,
        language_name: langMap[appState.language] || "",
        github_repo: appState.githubRepo,
        output_dir: appState.outputDir,
        ogg_path: appState.oggPath,
        wav_path: appState.wavPath,
      };

      const result = await runPipeline(config);
      appState.resultPath = result.notes_path;
      appState.createdIssues = result.created_issues;
      appState.phase = "result";
    } catch (e: any) {
      appState.errorMessage = e.toString();
      appState.phase = "error";
    } finally {
      unlisten();
    }
  }
</script>

<section
  class="rounded-lg p-5 border"
  style="background: var(--surface); border-color: var(--border)"
>
  <div class="flex items-center justify-between mb-4">
    <h2 class="text-lg font-semibold">Record</h2>
    {#if appState.phase === "recording"}
      <span class="text-2xl font-mono tabular-nums" style="color: var(--text)">
        {appState.formattedTime}
      </span>
    {/if}
  </div>

  <div class="flex flex-col items-center gap-4">
    <!-- Record button -->
    <button
      class="w-20 h-20 rounded-full flex items-center justify-center transition-all cursor-pointer border-0"
      style="background: {appState.phase === 'recording' ? 'var(--danger)' : 'var(--accent)'}; box-shadow: 0 0 {appState.phase === 'recording' ? '20px' : '0px'} {appState.phase === 'recording' ? 'var(--danger)' : 'transparent'}"
      onclick={handleRecord}
      disabled={appState.phase !== "setup" && appState.phase !== "recording"}
    >
      {#if appState.phase === "recording"}
        <!-- Stop icon -->
        <svg width="28" height="28" viewBox="0 0 24 24" fill="white">
          <rect x="6" y="6" width="12" height="12" rx="2" />
        </svg>
      {:else}
        <!-- Mic icon -->
        <svg width="28" height="28" viewBox="0 0 24 24" fill="white">
          <path
            d="M12 1a3 3 0 0 0-3 3v8a3 3 0 0 0 6 0V4a3 3 0 0 0-3-3zM19 10v2a7 7 0 0 1-14 0v-2H3v2a9 9 0 0 0 8 8.94V23h2v-2.06A9 9 0 0 0 21 12v-2h-2z"
          />
        </svg>
      {/if}
    </button>

    {#if appState.phase === "recording"}
      <AudioMeter level={appState.audioLevel} />
    {/if}

    {#if appState.phase === "setup"}
      <p class="text-sm" style="color: var(--text-muted)">
        Click to start recording
      </p>
    {/if}
  </div>
</section>
