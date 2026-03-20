<script lang="ts">
  interface Props {
    name: string;
    organization: string;
    enabled: boolean;
    ontoggle: () => void;
    onremove: () => void;
    disabled?: boolean;
  }

  let { name, organization, enabled, ontoggle, onremove, disabled = false }: Props = $props();
</script>

<span
  class="inline-flex items-center gap-1 px-3 py-1 rounded-full text-sm border cursor-pointer select-none transition-opacity"
  style="background: {enabled ? 'var(--accent)' : 'var(--bg)'}; border-color: {enabled ? 'var(--accent)' : 'var(--border)'}; color: {enabled ? 'white' : 'var(--text-muted)'}; opacity: {disabled ? 0.6 : 1}"
  role="button"
  tabindex="0"
  onclick={() => !disabled && ontoggle()}
  onkeydown={(e: KeyboardEvent) => e.key === "Enter" && !disabled && ontoggle()}
>
  <span>{name}</span>
  {#if organization}
    <span class="opacity-70 text-xs">({organization})</span>
  {/if}
  {#if !disabled}
    <button
      class="ml-1 text-xs opacity-60 hover:opacity-100 cursor-pointer"
      onclick={(e: MouseEvent) => { e.stopPropagation(); onremove(); }}
      aria-label="Remove {name}"
    >
      x
    </button>
  {/if}
</span>
