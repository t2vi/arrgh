<script lang="ts">
  import { LoaderCircle } from '@lucide/svelte'
  import { api } from '../../lib/api'
  import { cn } from '../../lib/utils'

  let {
    chapterId,
    total,
    onPageSeen,
    onLastPageFailed,
    initialPage,
    zoom = 100,
  }: {
    chapterId: string
    total: number | null
    onPageSeen: (page: number) => void
    onLastPageFailed: (lastGood: number) => void
    initialPage: number
    zoom?: number
  } = $props()

  // svelte-ignore state_referenced_locally
  const knownMax = total != null ? total : 200
  // svelte-ignore state_referenced_locally
  let rendered = $state(Math.min(knownMax, (total ?? 0) > 0 ? (total as number) : 20))
  let failed = $state<Set<number>>(new Set())
  let loaded = $state<Set<number>>(new Set())
  // Failures inside a known page count are transient (source proxy hiccup), not the end of
  // the chapter — retry them instead of dropping the page until a remount (GH #274).
  const MAX_AUTO_RETRIES = 3
  let retries = $state<Record<number, number>>({})
  let gaveUp = $state<Set<number>>(new Set())
  let containerEl: HTMLDivElement | undefined = $state()
  let seen = -1

  // Applies the saved reading position exactly once per chapter, when it first opens.
  // Gated on the chapter actually changing (not on the effect merely re-running) —
  // onScroll below reports the page the user is currently viewing back up through
  // onPageSeen, which flows back in as `initialPage`; an effect that treated every
  // `initialPage` change as "apply it" would re-force scrollTop on every scroll,
  // fighting the user (GH #172).
  let lastAppliedChapterId: string | undefined
  $effect(() => {
    if (initialPage <= 0 || !containerEl || chapterId === lastAppliedChapterId) return
    lastAppliedChapterId = chapterId
    containerEl.scrollTop = initialPage * 500
  })

  function onScroll(e: Event) {
    const el = e.currentTarget as HTMLDivElement
    if (el.scrollTop + el.clientHeight > el.scrollHeight - 400) {
      rendered = Math.min(knownMax, rendered + 5)
    }
    const imgs = el.querySelectorAll<HTMLImageElement>('[data-page]')
    let current = 0
    for (const img of imgs) {
      const rect = img.getBoundingClientRect()
      const containerTop = el.getBoundingClientRect().top
      if (rect.top - containerTop < el.clientHeight * 0.5) {
        current = Number(img.dataset.page)
      }
    }
    if (current !== seen) {
      seen = current
      onPageSeen(current)
      const completed = total != null && current >= total - 1
      api.updateProgress(chapterId, current, completed).catch(() => {})
    }
  }

  function onPageError(p: number) {
    if (total == null || p >= total) {
      loaded = new Set(loaded).add(p)
      if (p > 0) onLastPageFailed(p)
      failed = new Set(failed).add(p)
      return
    }
    const n = retries[p] ?? 0
    if (n >= MAX_AUTO_RETRIES) gaveUp = new Set(gaveUp).add(p)
    else setTimeout(() => retry(p), 1000 * (n + 1))
  }

  function retry(p: number) {
    const next = new Set(gaveUp)
    next.delete(p)
    gaveUp = next
    retries = { ...retries, [p]: (retries[p] ?? 0) + 1 }
  }

  const pages = $derived(Array.from({ length: rendered }, (_, i) => i))
</script>

<div bind:this={containerEl} class="flex-1 overflow-y-auto bg-black" onscroll={onScroll}>
  <div class="flex flex-col items-center gap-1 py-2 w-full mx-auto" style={`max-width: ${zoom * 8}px`}>
    {#each pages as p (p)}
      {#if !failed.has(p)}
        <div class="relative w-full">
          {#if gaveUp.has(p)}
            <div class="w-full flex flex-col items-center justify-center gap-2 bg-white/5 text-sm text-muted-foreground" style="min-height: 480px">
              <p>Page {p + 1} failed to load.</p>
              <button class="text-primary hover:underline" aria-label={`Retry page ${p + 1}`} onclick={() => retry(p)}>Retry</button>
            </div>
          {:else if !loaded.has(p)}
            <div class="w-full flex items-center justify-center bg-white/5" style="min-height: 480px">
              <LoaderCircle class="w-7 h-7 animate-spin text-muted-foreground/50" />
            </div>
          {/if}
          <img
            data-page={p}
            src={retries[p] ? `${api.pageUrl(chapterId, p)}?retry=${retries[p]}` : api.pageUrl(chapterId, p)}
            alt={`Page ${p + 1}`}
            class={cn('w-full block select-none', !loaded.has(p) && 'h-0 overflow-hidden')}
            onload={() => (loaded = new Set(loaded).add(p))}
            onerror={() => onPageError(p)}
          />
        </div>
      {/if}
    {/each}
  </div>
</div>
