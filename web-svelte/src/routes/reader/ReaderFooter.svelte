<script lang="ts">
  import { ChevronLeft, ChevronRight } from '@lucide/svelte'
  import Button from '../../lib/components/ui/Button.svelte'
  import { router } from '../../lib/router.svelte'
  import { ROUTES } from '../../lib/routes'
  import type { Chapter } from '../../lib/types'

  let {
    mode,
    page,
    totalLabel,
    atEnd,
    prevChapter,
    nextChapter,
    onPrevPage,
    onNextPage,
  }: {
    mode: 'paged' | 'scroll' | 'novel'
    page: number
    total: number | null
    totalLabel: string
    atEnd: boolean
    prevChapter: Chapter | null
    nextChapter: Chapter | null
    onPrevPage: () => void
    onNextPage: () => void
  } = $props()

  const atStart = $derived(page === 0)

  const prevAction = $derived(atStart && prevChapter ? () => router.navigate(ROUTES.reader(prevChapter.id)) : atStart ? null : onPrevPage)
  const nextAction = $derived(atEnd && nextChapter ? () => router.navigate(ROUTES.reader(nextChapter.id)) : atEnd ? null : onNextPage)
  const prevLabel = $derived(atStart && prevChapter ? 'Prev Ch.' : 'Prev')
  const nextLabel = $derived(atEnd && nextChapter ? 'Next Ch.' : 'Next')
</script>

{#if mode === 'paged'}
  <footer class="flex items-center justify-center gap-4 px-4 py-3 border-t border-border bg-card/90 backdrop-blur shrink-0">
    <Button variant="outline" size="sm" disabled={!prevAction} onclick={prevAction ?? undefined} class="gap-1">
      <ChevronLeft class="w-3 h-3" /> {prevLabel}
    </Button>
    <span class="text-sm text-muted-foreground min-w-16 text-center">{page + 1} / {totalLabel}</span>
    <Button variant="outline" size="sm" disabled={!nextAction} onclick={nextAction ?? undefined} class="gap-1">
      {nextLabel} <ChevronRight class="w-3 h-3" />
    </Button>
  </footer>
{:else}
  <footer class="flex items-center justify-center gap-4 px-4 py-3 border-t border-border bg-card/90 backdrop-blur shrink-0">
    <Button
      variant="outline"
      size="sm"
      disabled={!prevChapter}
      onclick={() => prevChapter && router.navigate(ROUTES.reader(prevChapter.id))}
      class="gap-1"
    >
      <ChevronLeft class="w-3 h-3" /> Prev Ch.
    </Button>
    <Button
      variant="outline"
      size="sm"
      disabled={!nextChapter}
      onclick={() => nextChapter && router.navigate(ROUTES.reader(nextChapter.id))}
      class="gap-1"
    >
      Next Ch. <ChevronRight class="w-3 h-3" />
    </Button>
  </footer>
{/if}
