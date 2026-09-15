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
  let containerEl: HTMLDivElement | undefined = $state()
  let seen = -1

  $effect(() => {
    if (initialPage <= 0 || !containerEl) return
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

  const pages = $derived(Array.from({ length: rendered }, (_, i) => i))
</script>

<div bind:this={containerEl} class="flex-1 overflow-y-auto bg-black" onscroll={onScroll}>
  <div class="flex flex-col items-center gap-1 py-2 w-full mx-auto" style={`max-width: ${zoom * 8}px`}>
    {#each pages as p (p)}
      {#if !failed.has(p)}
        <div class="relative w-full">
          {#if !loaded.has(p)}
            <div class="w-full flex items-center justify-center bg-white/5" style="min-height: 480px">
              <LoaderCircle class="w-7 h-7 animate-spin text-muted-foreground/50" />
            </div>
          {/if}
          <img
            data-page={p}
            src={api.pageUrl(chapterId, p)}
            alt={`Page ${p + 1}`}
            class={cn('w-full block select-none', !loaded.has(p) && 'h-0 overflow-hidden')}
            onload={() => (loaded = new Set(loaded).add(p))}
            onerror={() => {
              loaded = new Set(loaded).add(p)
              if (p > 0) onLastPageFailed(p)
              failed = new Set(failed).add(p)
            }}
          />
        </div>
      {/if}
    {/each}
  </div>
</div>
