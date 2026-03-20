<script lang="ts">
  import { onMount } from "svelte";
  import { appState } from "./lib/state.svelte";
  import { getDefaultOutputDir } from "./lib/api";
  import SetupSection from "./components/SetupSection.svelte";
  import RecordSection from "./components/RecordSection.svelte";
  import ProcessingSection from "./components/ProcessingSection.svelte";
  import ResultSection from "./components/ResultSection.svelte";

  onMount(async () => {
    try {
      appState.outputDir = await getDefaultOutputDir();
    } catch (e) {
      console.error("Failed to get output dir:", e);
    }
  });
</script>

<main class="max-w-2xl mx-auto p-8 min-h-screen">
  <header class="mb-8">
    <h1 class="text-3xl font-bold text-white">Meeting Scribe</h1>
    <p class="text-sm mt-1" style="color: var(--text-muted)">
      Record, transcribe, and summarize meetings
    </p>
  </header>

  <div class="space-y-6">
    <SetupSection />
    <RecordSection />
    {#if appState.phase === "processing" || appState.phase === "result" || appState.phase === "error"}
      <ProcessingSection />
    {/if}
    {#if appState.phase === "result"}
      <ResultSection />
    {/if}
    {#if appState.phase === "error"}
      <div
        class="rounded-lg p-4 border"
        style="background: #451a1a; border-color: var(--danger)"
      >
        <p class="font-semibold" style="color: var(--danger)">Error</p>
        <p class="text-sm mt-1" style="color: var(--text-muted)">
          {appState.errorMessage}
        </p>
        <button
          class="mt-3 px-4 py-2 rounded-md text-sm font-medium text-white cursor-pointer"
          style="background: var(--surface-hover)"
          onclick={() => appState.reset()}
        >
          Try Again
        </button>
      </div>
    {/if}
  </div>
</main>
