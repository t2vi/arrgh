<script lang="ts">
  import { cn } from '../utils'
  let {
    value,
    options,
    onChange,
  }: { value: string; options: { value: string; label: string }[]; onChange: (v: string) => void } = $props()
</script>

<div class="flex rounded-lg bg-muted p-0.5 gap-0.5">
  {#each options as o (o.value)}
    <button
      type="button"
      class={cn(
        'px-3 py-1 rounded-md text-xs font-semibold transition-colors',
        value === o.value ? 'bg-card text-foreground shadow-sm' : 'text-muted-foreground hover:text-foreground',
      )}
      onclick={() => onChange(o.value)}
    >
      {o.label}
    </button>
  {/each}
</div>
