<script lang="ts">
  import type { Snippet } from 'svelte'
  import { cn } from '../../utils'

  let {
    class: className = '',
    children,
    ...rest
  }: { class?: string; children?: Snippet } & Record<string, unknown> = $props()
</script>

<button
  class={cn(
    'inline-flex items-center justify-center gap-2 whitespace-nowrap rounded-md text-sm font-medium transition-colors focus-visible:outline-none focus-visible:ring-1 focus-visible:ring-ring disabled:pointer-events-none disabled:opacity-50 h-9 px-4 py-2 bg-primary text-primary-foreground shadow hover:bg-primary/90',
    className,
  )}
  {...rest}
>
  {@render children?.()}
</button>
