<script lang="ts">
  import { appState } from "../lib/state.svelte";
  import { openFile } from "../lib/api";

  async function handleOpen() {
    try {
      await openFile(appState.resultPath);
    } catch (e) {
      console.error("Failed to open file:", e);
    }
  }

  async function handleOpenUrl(url: string) {
    try {
      await openFile(url);
    } catch (e) {
      console.error("Failed to open URL:", e);
    }
  }
</script>

<section
  class="rounded-lg p-5 border"
  style="background: var(--surface); border-color: var(--success)"
>
  <div class="flex items-center gap-3 mb-3">
    <svg
      width="24"
      height="24"
      viewBox="0 0 24 24"
      fill="none"
      stroke="var(--success)"
      stroke-width="2"
    >
      <path d="M22 11.08V12a10 10 0 1 1-5.93-9.14" />
      <polyline points="22 4 12 14.01 9 11.01" />
    </svg>
    <h2 class="text-lg font-semibold" style="color: var(--success)">
      Meeting notes ready!
    </h2>
  </div>

  <!-- Notes file link -->
  <button
    class="w-full text-left rounded-md px-4 py-3 border cursor-pointer hover:opacity-90 transition-opacity"
    style="background: var(--bg); border-color: var(--border)"
    onclick={handleOpen}
  >
    <p class="text-sm font-medium" style="color: var(--accent)">
      {appState.resultPath.split("/").pop()}
    </p>
    <p class="text-xs mt-1" style="color: var(--text-muted)">
      {appState.resultPath}
    </p>
  </button>

  <!-- Created GitHub issues -->
  {#if appState.createdIssues.length > 0}
    <div class="mt-4">
      <h3 class="text-sm font-semibold mb-2 flex items-center gap-2">
        <svg width="16" height="16" viewBox="0 0 16 16" fill="var(--text-muted)">
          <path d="M8 0c4.42 0 8 3.58 8 8a8.013 8.013 0 0 1-5.45 7.59c-.4.08-.55-.17-.55-.38 0-.27.01-1.13.01-2.2 0-.75-.25-1.23-.54-1.48 1.78-.2 3.65-.88 3.65-3.95 0-.88-.31-1.59-.82-2.15.08-.2.36-1.02-.08-2.12 0 0-.67-.22-2.2.82-.64-.18-1.32-.27-2-.27-.68 0-1.36.09-2 .27-1.53-1.03-2.2-.82-2.2-.82-.44 1.1-.16 1.92-.08 2.12-.51.56-.82 1.28-.82 2.15 0 3.06 1.86 3.75 3.64 3.95-.23.2-.44.55-.51 1.07-.46.21-1.61.55-2.33-.66-.15-.24-.6-.83-1.23-.82-.67.01-.27.38.01.53.34.19.73.9.82 1.13.16.45.68 1.31 2.69.94 0 .67.01 1.3.01 1.49 0 .21-.15.45-.55.38A7.995 7.995 0 0 1 0 8c0-4.42 3.58-8 8-8Z" />
        </svg>
        Issues Created ({appState.createdIssues.length})
      </h3>
      <div class="space-y-1">
        {#each appState.createdIssues as issue (issue.number)}
          <button
            class="w-full text-left rounded px-3 py-2 text-sm cursor-pointer hover:opacity-80 transition-opacity"
            style="background: var(--bg)"
            onclick={() => handleOpenUrl(issue.url)}
          >
            <span style="color: var(--text-muted)">#{issue.number}</span>
            <span class="ml-1">{issue.title}</span>
          </button>
        {/each}
      </div>
    </div>
  {/if}

  <button
    class="mt-4 w-full px-4 py-2 rounded-md text-sm font-medium text-white cursor-pointer"
    style="background: var(--accent)"
    onclick={() => appState.reset()}
  >
    New Recording
  </button>
</section>
