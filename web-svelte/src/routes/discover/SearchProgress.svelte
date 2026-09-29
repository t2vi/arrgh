<script lang="ts">
  import { Loader2 } from '@lucide/svelte'
  import Skeleton from '../../lib/components/ui/Skeleton.svelte'
  import { cn } from '../../lib/utils'
  import type { SourceInfo } from '../../lib/api'
  import type { SourceProgress } from '../../lib/discover.svelte'

  let {
    sources,
    state,
    skeleton = false,
    isFetching = false,
  }: {
    sources: SourceInfo[]
    state: Map<string, SourceProgress>
    skeleton?: boolean
    isFetching?: boolean
  } = $props()

  const TITLES: Record<string, string> = { empty: 'no results', error: 'error', timeout: 'timed out' }

  const statusOf = (key: string) => state.get(key)?.status ?? 'searching'
  // isFetching, not just the per-authority pills: the source-fallback leg (spec 036) keeps
  // running after every authority pill has settled, with no pill of its own.
  const anySearching = $derived(isFetching || sources.some((s) => statusOf(s.key) === 'searching'))
</script>

<div class="space-y-4">
  <div class="rounded-lg border border-border bg-card px-4 py-3">
    <p class="flex items-center gap-1.5 text-xs font-medium text-muted-foreground uppercase tracking-wider mb-3">
      {#if anySearching}
        <Loader2 data-testid="search-spinner" class="w-3.5 h-3.5 animate-spin" />
      {/if}
      {anySearching ? 'Searching sources…' : 'Results from…'}
    </p>
    <div class="flex flex-wrap gap-2">
      {#each sources as s (s.key)}
        {@const status = statusOf(s.key)}
        {@const failed = status === 'error' || status === 'timeout'}
        <span
          data-testid={`source-pill-${s.key}`}
          title={TITLES[status]}
          class={cn(
            'flex items-center gap-1.5 rounded-full border px-2.5 py-1 text-xs font-medium transition-all duration-300',
            status === 'searching' && 'border-primary/40 bg-primary/10 text-primary',
            status === 'found' && 'border-green-500/40 bg-green-500/10 text-green-400',
            status === 'empty' && 'border-border bg-muted text-muted-foreground opacity-50',
            failed && 'border-amber-500/40 bg-amber-500/10 text-amber-400',
          )}
        >
          <span
            data-testid="source-dot"
            class={cn(
              'w-1.5 h-1.5 rounded-full',
              status === 'searching' && 'bg-primary animate-pulse',
              status === 'found' && 'bg-green-400',
              status === 'empty' && 'bg-muted-foreground/40',
              failed && 'bg-amber-400',
            )}
          ></span>
          {s.label}
          {#if status === 'found'}<span class="opacity-70">· {state.get(s.key)?.count}</span>{/if}
        </span>
      {/each}
    </div>
  </div>

  {#if skeleton}
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
