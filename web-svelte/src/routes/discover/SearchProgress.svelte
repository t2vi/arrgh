<script lang="ts">
  import Skeleton from '../../lib/components/ui/Skeleton.svelte'
  import { cn } from '../../lib/utils'

  const SOURCES = [
    { key: 'mangaupdates', label: 'MangaUpdates' },
    { key: 'anilist', label: 'AniList' },
    { key: 'mangadex', label: 'MangaDex' },
    { key: 'novelupdates', label: 'NovelUpdates' },
    { key: 'wuxiaworld', label: 'WuxiaWorld' },
    { key: 'nhentai', label: 'nhentai' },
  ]

  let { completedSources }: { completedSources?: Set<string> } = $props()

  const isDone = $derived(completedSources !== undefined)

  let visible = $state(0)
  let settled = $state(0)

  // Stagger purple pills appearing while searching
  $effect(() => {
    if (isDone || visible >= SOURCES.length) return
    const t = setTimeout(() => (visible += 1), 120)
    return () => clearTimeout(t)
  })

  // Stagger green/grey transition when results arrive
  $effect(() => {
    if (!isDone) {
      settled = 0
      return
    }
    if (settled >= SOURCES.length) return
    const t = setTimeout(() => (settled += 1), 100)
    return () => clearTimeout(t)
  })
</script>

<div class="space-y-4">
  <div class="rounded-lg border border-border bg-card px-4 py-3">
    <p class="text-xs font-medium text-muted-foreground uppercase tracking-wider mb-3">
      {isDone ? 'Results from…' : 'Searching sources…'}
    </p>
    <div class="flex flex-wrap gap-2">
      {#each SOURCES as s, i (s.key)}
        {@const isVisible = isDone || i < visible}
        {@const hasSettled = isDone && i < settled}
        {@const hasResult = hasSettled && completedSources!.has(s.key)}
        <span
          data-testid={`source-pill-${s.key}`}
          class={cn(
            'flex items-center gap-1.5 rounded-full border px-2.5 py-1 text-xs font-medium transition-all duration-300',
            !isVisible && 'opacity-0 translate-y-1',
            isVisible && !hasSettled && 'border-primary/40 bg-primary/10 text-primary',
            hasSettled && hasResult && 'border-green-500/40 bg-green-500/10 text-green-400',
            hasSettled && !hasResult && 'border-border bg-muted text-muted-foreground opacity-50',
          )}
        >
          <span
            data-testid="source-dot"
            class={cn(
              'w-1.5 h-1.5 rounded-full',
              isVisible && !hasSettled && 'bg-primary animate-pulse',
              hasSettled && hasResult && 'bg-green-400',
              hasSettled && !hasResult && 'bg-muted-foreground/40',
            )}
          ></span>
          {s.label}
        </span>
      {/each}
    </div>
  </div>

  {#if !isDone}
    <div data-testid="skeleton-rows" class="space-y-3">
      {#each Array.from({ length: 4 }) as _, i (i)}
        <div class="flex gap-3 rounded-lg border border-border bg-card p-3">
          <Skeleton class="w-14 shrink-0 rounded aspect-[2/3]" />
          <div class="flex-1 space-y-2">
            <Skeleton class="h-4 w-3/4" />
            <Skeleton class="h-3 w-1/4" />
            <Skeleton class="h-3 w-full" />
            <Skeleton class="h-3 w-5/6" />
          </div>
        </div>
      {/each}
    </div>
  {/if}
</div>
