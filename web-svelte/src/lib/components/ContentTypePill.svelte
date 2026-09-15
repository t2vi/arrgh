<script lang="ts">
  import { cn } from '../utils'

  const COLORS: Record<string, { bg: string; border: string; text: string }> = {
    manga: { bg: 'bg-violet-500/20', border: 'border-violet-500/30', text: 'text-violet-400' },
    manhwa: { bg: 'bg-sky-500/20', border: 'border-sky-500/30', text: 'text-sky-400' },
    manhua: { bg: 'bg-amber-500/20', border: 'border-amber-500/30', text: 'text-amber-400' },
    novel: { bg: 'bg-emerald-500/20', border: 'border-emerald-500/30', text: 'text-emerald-400' },
  }

  let { type, size = 'md' }: { type: string; size?: 'sm' | 'md' } = $props()
  const c = $derived(COLORS[type] ?? COLORS.manga)
  const textCls = $derived(size === 'sm' ? 'text-[10px]' : 'text-[11px]')
</script>

<span class={cn(textCls, 'px-2 py-0.5 rounded-full border font-semibold capitalize', c.bg, c.border, c.text)}>
  {type}
</span>
