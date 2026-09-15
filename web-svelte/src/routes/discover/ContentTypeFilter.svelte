<script lang="ts">
  import { cn } from '../../lib/utils'

  const CONTENT_TYPES = [
    { value: 'manga', label: 'Manga' },
    { value: 'manhwa', label: 'Manhwa' },
    { value: 'manhua', label: 'Manhua' },
    { value: 'one-shot', label: 'One-shot' },
    { value: 'novel', label: 'Novel' },
    { value: 'hentai', label: 'Hentai' },
  ] as const

  let {
    value,
    onChange,
    availableTypes,
  }: {
    value: string | undefined
    onChange: (v: string | undefined) => void
    availableTypes: Set<string>
  } = $props()

  const visible = $derived(CONTENT_TYPES.filter((ct) => availableTypes.has(ct.value)))
</script>

{#if visible.length > 0}
  <div class="flex gap-1 flex-wrap">
    <button
      type="button"
      onclick={() => onChange(undefined)}
      class={cn(
        'px-2.5 py-0.5 rounded-full text-xs font-medium transition-colors border',
        value === undefined
          ? 'bg-primary text-primary-foreground border-primary'
          : 'bg-muted text-muted-foreground border-transparent hover:border-border hover:text-foreground',
      )}
    >
      All
    </button>
    {#each visible as ct (ct.value)}
      <button
        type="button"
        onclick={() => onChange(ct.value)}
        class={cn(
          'px-2.5 py-0.5 rounded-full text-xs font-medium transition-colors border',
          value === ct.value
            ? 'bg-primary text-primary-foreground border-primary'
            : 'bg-muted text-muted-foreground border-transparent hover:border-border hover:text-foreground',
        )}
      >
        {ct.label}
      </button>
    {/each}
  </div>
{/if}
