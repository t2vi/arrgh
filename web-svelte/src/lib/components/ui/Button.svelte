<script lang="ts">
  import type { Snippet } from 'svelte'
  import { cn } from '../../utils'

  const VARIANTS = {
    default: 'bg-primary text-primary-foreground shadow hover:bg-primary/90',
    outline: 'border border-input bg-background shadow-sm hover:bg-accent hover:text-accent-foreground',
  }
  const SIZES = {
    default: 'h-9 px-4 py-2',
    sm: 'h-7 rounded-md px-3 text-xs',
  }

  let {
    class: className = '',
    variant = 'default',
    size = 'default',
    children,
    ...rest
  }: {
    class?: string
    variant?: keyof typeof VARIANTS
    size?: keyof typeof SIZES
    children?: Snippet
  } & Record<string, unknown> = $props()
</script>

<button
  class={cn(
    'inline-flex items-center justify-center gap-2 whitespace-nowrap rounded-md text-sm font-medium transition-colors focus-visible:outline-none focus-visible:ring-1 focus-visible:ring-ring disabled:pointer-events-none disabled:opacity-50',
    SIZES[size],
    VARIANTS[variant],
    className,
  )}
  {...rest}
>
  {@render children?.()}
</button>
