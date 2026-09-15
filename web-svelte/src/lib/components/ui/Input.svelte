<script lang="ts">
  import { cn } from '../../utils'

  let {
    class: className = '',
    value = $bindable(''),
    ...rest
  }: { class?: string; value?: string } & Record<string, unknown> = $props()
</script>

<input
  bind:value
  class={cn(
    'flex h-9 w-full rounded-md border border-input bg-transparent px-3 py-1 text-sm shadow-sm transition-colors placeholder:text-muted-foreground focus-visible:outline-none focus-visible:ring-1 focus-visible:ring-ring disabled:cursor-not-allowed disabled:opacity-50',
    className,
  )}
  {...rest}
/>
