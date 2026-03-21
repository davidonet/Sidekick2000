<script lang="ts">
  import { onMount } from "svelte";
  import { appState } from "../lib/state.svelte";
  import {
    listInputDevices,
    saveInputDevice,
    startRecording,
    stopRecording,
    getAudioLevel,
    getElapsed,
    runPipeline,
    onPipelineProgress,
  } from "../lib/api";
  import type { PipelineConfig } from "../lib/types";
  import AudioMeter from "./AudioMeter.svelte";
  import Select from "./Select.svelte";

  let pollingId: ReturnType<typeof setInterval> | null = null;
  let stopping = $state(false);

  onMount(async () => {
    try {
      appState.inputDevices = await listInputDevices();
    } catch {
      // ignore — device list just stays empty
    }
  });

  async function handleRecord() {
    if (appState.phase === "recording") {
      // Stop recording
      stopping = true;
      if (pollingId) {
        clearInterval(pollingId);
        pollingId = null;
      }
      try {
        const [oggPath, wavPath] = await stopRecording();
        appState.oggPath = oggPath;
        appState.wavPath = wavPath;
        stopping = false;
        appState.phase = "processing";
        await startPipeline();
      } catch (e: any) {
        stopping = false;
        appState.errorMessage = e.toString();
        appState.phase = "error";
      }
    } else {
      // Start recording
      try {
        await startRecording(appState.selectedDevice || undefined);
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
    const unlisten = await onPipelineProgress((step, progress) => {
      appState.pipelineStep = step;
      appState.pipelineProgress = progress;
    });

    try {
      const langMap: Record<string, string> = {
        fr: "French",
        en: "English",
        es: "Spanish",
        de: "German",
        it: "Italian",
        pt: "Portuguese",
      };

      const config: PipelineConfig = {
        context: appState.contextLabel,
        context_content: appState.contextContent,
        speakers: appState.enabledSpeakers.map((s) => ({
          name: s.name,
          organization: s.organization,
        })),
        language_code: appState.language,
        language_name: langMap[appState.language] || "",
        github_repo: appState.githubRepo,
        output_dir: appState.outputDir,
        working_folder: appState.workingFolder,
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
    <!-- Device selector -->
    {#if appState.phase === "setup" && appState.inputDevices.length > 0}
      <div class="w-full">
        <label class="block text-xs mb-1" style="color: var(--text-muted)">
          Input device
        </label>
        <Select
          bind:value={appState.selectedDevice}
          onchange={() => saveInputDevice(appState.selectedDevice).catch(() => {})}
        >
          <option value="">Default</option>
          {#each appState.inputDevices as device}
            <option value={device}>{device}</option>
          {/each}
        </Select>
      </div>
    {/if}

    <!-- Record button -->
    <button
      class="w-20 h-20 rounded-full flex items-center justify-center transition-all border-0"
      class:cursor-pointer={!stopping}
      class:cursor-default={stopping}
      style="background: {appState.phase === 'recording' || stopping ? 'var(--danger)' : 'var(--accent)'}; opacity: {stopping ? 0.6 : 1}; box-shadow: 0 0 {appState.phase === 'recording' && !stopping ? '20px' : '0px'} {appState.phase === 'recording' ? 'var(--danger)' : 'transparent'}"
      onclick={handleRecord}
      disabled={stopping || (appState.phase !== "setup" && appState.phase !== "recording")}
    >
      {#if stopping}
        <!-- Spinner -->
        <svg class="animate-spin" width="28" height="28" viewBox="0 0 24 24" fill="none" stroke="white" stroke-width="2.5">
          <path d="M12 2a10 10 0 0 1 10 10" stroke-linecap="round" />
        </svg>
      {:else if appState.phase === "recording"}
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

    {#if stopping}
      <p class="text-sm" style="color: var(--text-muted)">Saving audio…</p>
    {:else if appState.phase === "recording"}
      <AudioMeter level={appState.audioLevel} />
    {:else if appState.phase === "setup"}
      <p class="text-sm" style="color: var(--text-muted)">
        Click to start recording
      </p>
    {/if}
  </div>
</section>
