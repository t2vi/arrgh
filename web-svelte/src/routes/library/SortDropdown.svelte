<script lang="ts">
  import { ChevronDown } from '@lucide/svelte'
  import Button from '../../lib/components/ui/Button.svelte'
  import type { SortOption } from '../../lib/library.svelte'

  const SORT_LABELS: Record<SortOption, string> = {
    recent: 'Recently Added',
    title_asc: 'A → Z',
    title_desc: 'Z → A',
    year: 'Year',
  }

  let { value, onChange }: { value: SortOption; onChange: (v: SortOption) => void } = $props()

  let open = $state(false)
  let ref: HTMLDivElement | undefined = $state()

  $effect(() => {
    if (!open) return
    function onMouseDown(e: MouseEvent) {
      if (!ref?.contains(e.target as Node)) open = false
    }
    document.addEventListener('mousedown', onMouseDown)
    return () => document.removeEventListener('mousedown', onMouseDown)
  })
</script>

<div bind:this={ref} class="relative">
  <Button variant="outline" size="sm" class="gap-1.5 text-xs" onclick={() => (open = !open)}>
    <span class="text-muted-foreground">Sort by:</span>
    {SORT_LABELS[value]}
    <ChevronDown class={`w-3 h-3 text-muted-foreground transition-transform ${open ? 'rotate-180' : ''}`} />
  </Button>
  {#if open}
    <div class="absolute right-0 top-full mt-1 w-44 rounded-md border border-border bg-background shadow-md z-20">
      {#each Object.keys(SORT_LABELS) as opt (opt)}
        <button
          class={`w-full text-left px-3 py-2 text-sm transition-colors first:rounded-t-md last:rounded-b-md hover:bg-muted ${
            value === opt ? 'font-medium text-foreground' : 'text-muted-foreground'
          }`}
          onclick={() => {
            onChange(opt as SortOption)
            open = false
          }}
        >
          {SORT_LABELS[opt as SortOption]}
        </button>
      {/each}
    </div>
  {/if}
</div>
