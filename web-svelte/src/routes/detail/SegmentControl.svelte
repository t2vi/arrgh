<script lang="ts">
  import { cn } from '../../lib/utils'

  let { options, active, onSelect }: { options: string[]; active: number; onSelect: (i: number) => void } = $props()
</script>

<div class="flex rounded-lg bg-muted p-0.5 gap-0.5">
  {#each options as label, i (label)}
    <button
      onclick={() => onSelect(i)}
      class={cn(
        'flex-1 text-[11px] font-medium py-1.5 rounded-md transition-colors',
        active === i ? 'bg-card text-foreground shadow-sm' : 'text-muted-foreground hover:text-foreground',
      )}
    >
      {label}
    </button>
  {/each}
</div>
