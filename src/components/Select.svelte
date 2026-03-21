<script lang="ts">
  import type { Snippet } from "svelte";

  let {
    value = $bindable(),
    disabled = false,
    onchange,
    children,
  }: {
    value?: string;
    disabled?: boolean;
    onchange?: (e: Event) => void;
    children?: Snippet;
  } = $props();
</script>

<div class="select-wrapper" class:disabled>
  <select bind:value {disabled} {onchange}>
    {@render children?.()}
  </select>
  <div class="chevron" aria-hidden="true">
    <svg width="13" height="13" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5" stroke-linecap="round" stroke-linejoin="round">
      <polyline points="6 9 12 15 18 9" />
    </svg>
  </div>
</div>

<style>
  .select-wrapper {
    position: relative;
    width: 100%;
  }

  select {
    width: 100%;
    appearance: none;
    -webkit-appearance: none;
    background: var(--bg);
    border: 1.5px solid var(--border);
    border-radius: 8px;
    color: var(--text);
    padding: 0.5rem 2.25rem 0.5rem 0.75rem;
    font-size: 0.875rem;
    line-height: 1.25rem;
    cursor: pointer;
    outline: none;
    transition: border-color 120ms ease, box-shadow 120ms ease;
  }

  select:hover:not(:disabled) {
    border-color: color-mix(in srgb, var(--accent) 60%, var(--border));
  }

  select:focus {
    border-color: var(--accent);
    box-shadow: 0 0 0 3px color-mix(in srgb, var(--accent) 20%, transparent);
  }

  .select-wrapper.disabled select {
    opacity: 0.45;
    cursor: not-allowed;
  }

  .chevron {
    position: absolute;
    right: 0.625rem;
    top: 50%;
    transform: translateY(-50%);
    color: var(--text-muted);
    pointer-events: none;
    display: flex;
    align-items: center;
    transition: color 120ms ease, transform 120ms ease;
  }

  select:focus ~ .chevron {
    color: var(--accent);
  }
</style>
