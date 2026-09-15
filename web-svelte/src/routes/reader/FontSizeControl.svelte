<script lang="ts">
  import Button from '../../lib/components/ui/Button.svelte'

  const SIZES = [14, 16, 18, 21] as const

  let { size, onApply }: { size: number; onApply: (size: number) => void } = $props()

  let open = $state(false)
</script>

<div class="relative">
  <Button variant="ghost" size="icon" onclick={() => (open = !open)} title="Font size" class="font-semibold text-sm">Aa</Button>
  {#if open}
    <div class="fixed inset-0 z-10" onclick={() => (open = false)} role="presentation"></div>
    <div class="absolute right-0 top-full mt-1 z-20 flex gap-1 p-2 rounded-lg bg-card border border-border shadow-lg">
      {#each SIZES as s (s)}
        <button
          type="button"
          onclick={() => {
            onApply(s)
            open = false
          }}
          class={`px-2.5 py-1 rounded-md text-xs font-medium transition-colors ${
            size === s ? 'bg-primary text-primary-foreground' : 'hover:bg-muted text-muted-foreground hover:text-foreground'
          }`}
        >
          {s}
        </button>
      {/each}
    </div>
  {/if}
</div>
