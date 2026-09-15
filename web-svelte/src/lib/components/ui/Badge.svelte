<script lang="ts">
  import { cn } from '../../utils'

  const VARIANTS = {
    default: 'border-transparent bg-primary text-primary-foreground',
    secondary: 'border-transparent bg-secondary text-secondary-foreground',
  }

  let {
    class: className = '',
    variant = 'default',
    children,
  }: {
    class?: string
    variant?: keyof typeof VARIANTS
    children?: import('svelte').Snippet
  } = $props()
</script>

<div
  class={cn(
    'inline-flex items-center rounded-md border px-2 py-0.5 text-xs font-semibold transition-colors',
    VARIANTS[variant],
    className,
  )}
>
  {@render children?.()}
</div>
