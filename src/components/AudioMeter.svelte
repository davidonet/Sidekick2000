<script lang="ts">
  interface Props {
    level: number;
  }

  let { level }: Props = $props();

  // Scale the level for better visibility (raw RMS is often quite small)
  let displayLevel = $derived(Math.min(1, level * 8));

  // Generate bar widths for the VU meter visualization
  let bars = $derived(
    Array.from({ length: 20 }, (_, i) => {
      const threshold = i / 20;
      return displayLevel > threshold;
    }),
  );
</script>

<div class="w-full max-w-xs">
  <div class="flex gap-0.5 h-6 items-end">
    {#each bars as active, i}
      <div
        class="flex-1 rounded-sm transition-all duration-75"
        style="height: {40 + i * 3}%; background: {active
          ? i > 15
            ? 'var(--danger)'
            : i > 10
              ? '#eab308'
              : 'var(--success)'
          : 'var(--border)'}"
      ></div>
    {/each}
  </div>
</div>
