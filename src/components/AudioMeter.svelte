<script lang="ts">
  interface Props {
    level: number;
    label?: string;
  }

  let { level, label = "" }: Props = $props();

  // Scale the level for better visibility (raw RMS is often quite small)
  let displayLevel = $derived(Math.min(1, level * 8));

  // 16 bars stacking from bottom (index 0) to top (index 15)
  let bars = $derived(
    Array.from({ length: 16 }, (_, i) => {
      const threshold = i / 16;
      return displayLevel > threshold;
    }),
  );
</script>

<div class="flex flex-col items-center gap-1.5">
  <!-- Vertical bar column: flex-col-reverse renders index 0 at the bottom -->
  <div class="flex flex-col-reverse gap-0.5" style="width: 20px; height: 80px;">
    {#each bars as active, i}
      <div
        class="flex-1 rounded-sm transition-all duration-75"
        style="background: {active
          ? i >= 13
            ? 'var(--danger)'
            : i >= 9
              ? '#eab308'
              : 'var(--success)'
          : 'var(--border)'}"
      ></div>
    {/each}
  </div>
  {#if label}
    <span
      class="text-center font-mono truncate"
      style="font-size: 10px; color: var(--text-muted); max-width: 72px;"
      title={label}
    >{label}</span>
  {/if}
</div>
