<script lang="ts">
  import { onMount } from "svelte";
  import { appState, settingsState } from "./lib/state.svelte";
  import SetupSection from "./components/SetupSection.svelte";
  import RecordSection from "./components/RecordSection.svelte";
  import ProcessingSection from "./components/ProcessingSection.svelte";
  import ResultSection from "./components/ResultSection.svelte";
  import SettingsSection from "./components/SettingsSection.svelte";

  let showSettings = $state(false);

  onMount(async () => {
    await settingsState.load();
    await appState.loadFromSettings();
  });

  function toggleSettings() {
    showSettings = !showSettings;
  }
</script>

<main class="max-w-2xl mx-auto p-8 min-h-screen">
  <header class="mb-8 flex items-center justify-between">
    <div>
      <h1 class="text-3xl font-bold text-white">Sidekick2000</h1>
      <p class="text-sm mt-1" style="color: var(--text-muted)">
        Record, transcribe, and summarize meetings
      </p>
    </div>
    <button
      class="p-2 rounded-lg cursor-pointer transition-opacity hover:opacity-80"
      style="background: {showSettings ? 'var(--accent)' : 'var(--surface)'}; color: var(--text)"
      onclick={toggleSettings}
      title="Settings"
    >
      <svg width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
        <circle cx="12" cy="12" r="3" />
        <path d="M19.4 15a1.65 1.65 0 0 0 .33 1.82l.06.06a2 2 0 1 1-2.83 2.83l-.06-.06a1.65 1.65 0 0 0-1.82-.33 1.65 1.65 0 0 0-1 1.51V21a2 2 0 1 1-4 0v-.09A1.65 1.65 0 0 0 9 19.4a1.65 1.65 0 0 0-1.82.33l-.06.06a2 2 0 1 1-2.83-2.83l.06-.06A1.65 1.65 0 0 0 4.68 15a1.65 1.65 0 0 0-1.51-1H3a2 2 0 1 1 0-4h.09A1.65 1.65 0 0 0 4.6 9a1.65 1.65 0 0 0-.33-1.82l-.06-.06a2 2 0 1 1 2.83-2.83l.06.06A1.65 1.65 0 0 0 9 4.68a1.65 1.65 0 0 0 1-1.51V3a2 2 0 1 1 4 0v.09a1.65 1.65 0 0 0 1 1.51 1.65 1.65 0 0 0 1.82-.33l.06-.06a2 2 0 1 1 2.83 2.83l-.06.06A1.65 1.65 0 0 0 19.4 9a1.65 1.65 0 0 0 1.51 1H21a2 2 0 1 1 0 4h-.09a1.65 1.65 0 0 0-1.51 1z" />
      </svg>
    </button>
  </header>

  {#if showSettings}
    <SettingsSection />
  {:else}
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
  {/if}
</main>
