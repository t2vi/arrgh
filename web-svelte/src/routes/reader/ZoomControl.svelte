<script lang="ts">
  import { ZoomIn } from '@lucide/svelte'
  import Button from '../../lib/components/ui/Button.svelte'

  const LEVELS = [50, 75, 100, 125, 150] as const

  let { zoom, onApply }: { zoom: number; onApply: (zoom: number) => void } = $props()

  let open = $state(false)
</script>

<div class="relative">
  <Button variant="ghost" size="icon" onclick={() => (open = !open)} title="Zoom">
    <ZoomIn class="w-4 h-4" />
  </Button>
  {#if open}
    <div class="fixed inset-0 z-10" onclick={() => (open = false)} role="presentation"></div>
    <div class="absolute right-0 top-full mt-1 z-20 flex gap-1 p-2 rounded-lg bg-card border border-border shadow-lg">
      {#each LEVELS as l (l)}
        <button
          type="button"
          onclick={() => {
            onApply(l)
            open = false
          }}
          class={`px-2.5 py-1 rounded-md text-xs font-medium transition-colors ${
            zoom === l ? 'bg-primary text-primary-foreground' : 'hover:bg-muted text-muted-foreground hover:text-foreground'
          }`}
        >
          {l}%
        </button>
      {/each}
    </div>
  {/if}
</div>
