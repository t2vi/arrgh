<script lang="ts">
  import { ChevronRight, Library, LoaderCircle, AlignJustify, BookOpen } from '@lucide/svelte'
  import { api } from '../lib/api'
  import { router } from '../lib/router.svelte'
  import { ROUTES } from '../lib/routes'
  import Button from '../lib/components/ui/Button.svelte'
  import { ReaderStore } from '../lib/reader.svelte'
  import { createNovelFontSize, createImageZoom } from '../lib/readerPrefs.svelte'
  import ScrollReader from './reader/ScrollReader.svelte'
  import NovelReader from './reader/NovelReader.svelte'
  import ReaderFooter from './reader/ReaderFooter.svelte'
  import ProgressBar from './reader/ProgressBar.svelte'
  import FontSizeControl from './reader/FontSizeControl.svelte'
  import ZoomControl from './reader/ZoomControl.svelte'

  let params: Record<string, string> = $props()
  const chapterId = $derived(params.id)

  const store = new ReaderStore()
  $effect(() => store.load(chapterId))

  const fontSizePref = createNovelFontSize()
  const zoomPref = createImageZoom()

  let imgLoading = $state(true)
  $effect(() => {
    void store.page
    imgLoading = true
  })

  const footerMode = $derived(store.isNovel ? 'novel' : store.effectiveMode)
</script>

{#if !store.chapter}
  <div class="flex items-center justify-center h-full text-muted-foreground">Loading…</div>
{:else}
  {@const chapter = store.chapter}
  <div class="reader-page">
    <header class="relative z-10 flex items-center gap-2 px-4 py-3 border-b border-border bg-card/90 backdrop-blur shrink-0">
      <Button variant="ghost" size="icon" onclick={() => router.navigate(ROUTES.title(chapter.title_id))} title="Back">
        <ChevronRight class="w-4 h-4 rotate-180" />
      </Button>
      <Button variant="ghost" size="icon" onclick={() => router.navigate(ROUTES.home)} title="Home">
        <Library class="w-4 h-4" />
      </Button>
      <p class="text-sm font-medium flex-1 truncate">
        Ch. {chapter.number}{chapter.title ? ` — ${chapter.title}` : ''}
      </p>
      {#if store.isNovel}
        <FontSizeControl size={fontSizePref.size} onApply={(s) => fontSizePref.apply(s)} />
      {:else}
        <span class="text-xs text-muted-foreground shrink-0 mr-1">
          {store.effectiveMode === 'paged' ? `${store.page + 1} / ${store.totalLabel}` : store.totalLabel !== '?' ? `${store.totalLabel} pages` : ''}
        </span>
        <ZoomControl zoom={zoomPref.zoom} onApply={(z) => zoomPref.apply(z)} />
        <Button variant="ghost" size="icon" onclick={() => store.toggleMode()} title={store.effectiveMode === 'paged' ? 'Switch to scroll' : 'Switch to paged'}>
          {#if store.effectiveMode === 'scroll'}
            <BookOpen class="w-4 h-4" />
          {:else}
            <AlignJustify class="w-4 h-4" />
          {/if}
        </Button>
      {/if}
    </header>

    <ProgressBar value={store.progress} />

    {#if store.isNovel}
      {#if store.novelContent != null}
        <NovelReader
          content={store.novelContent}
          fontSize={fontSizePref.size}
          onRead={() => store.markNovelRead()}
          onProgress={(pct) => store.setNovelProgress(pct)}
        />
      {:else if store.novelError}
        <div class="flex flex-col items-center justify-center h-full gap-3 text-center px-6">
          <p class="text-muted-foreground text-sm">Chapter not downloaded.</p>
          <p class="text-muted-foreground/60 text-xs">Go back to the manga page and download this chapter first.</p>
          <button onclick={() => window.history.back()} class="mt-2 text-sm text-primary hover:underline">Go back</button>
        </div>
      {:else}
        <div class="flex items-center justify-center h-full text-muted-foreground">Loading…</div>
      {/if}
    {:else if store.effectiveMode === 'paged'}
      <!-- Click-to-advance handled here, not on the img — a click on the img bubbles up to this same handler. -->
      <div class="reader-scroll" onclick={() => store.goTo(store.page + 1)} role="presentation">
        {#if imgLoading}
          <div class="absolute inset-0 flex items-center justify-center pointer-events-none">
            <LoaderCircle class="w-8 h-8 animate-spin text-muted-foreground/60" />
          </div>
        {/if}
        <img
          src={api.pageUrl(chapterId, store.page)}
          alt={`Page ${store.page + 1}`}
          style={`max-width: ${zoomPref.zoom * 8}px`}
          onload={() => (imgLoading = false)}
          onerror={() => {
            imgLoading = false
            if (store.page > 0) store.setLastPage(store.page)
          }}
        />
      </div>
    {:else}
      <ScrollReader
        chapterId={chapterId}
        total={store.total}
        onPageSeen={(p) => store.goTo(p)}
        onLastPageFailed={(p) => store.setLastPage(p)}
        initialPage={store.page}
        zoom={zoomPref.zoom}
      />
    {/if}

    <ReaderFooter
      mode={footerMode}
      page={store.page}
      total={store.total}
      totalLabel={store.totalLabel}
      atEnd={store.atEnd}
      prevChapter={store.prevChapter}
      nextChapter={store.nextChapter}
      onPrevPage={() => store.goTo(store.page - 1)}
      onNextPage={() => store.goTo(store.page + 1)}
    />
  </div>
{/if}
