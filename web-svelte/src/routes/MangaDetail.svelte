<script lang="ts">
  import {
    Download,
    RefreshCw,
    LoaderCircle,
    X,
    ArrowUp,
    ArrowDown,
    Trash2,
    ChevronDown,
    AlertTriangle,
    Terminal,
  } from '@lucide/svelte'
  import { api, isAdmin } from '../lib/api'
  import Button from '../lib/components/ui/Button.svelte'
  import ContentTypePill from '../lib/components/ContentTypePill.svelte'
  import { router } from '../lib/router.svelte'
  import { ROUTES } from '../lib/routes'
  import { cn } from '../lib/utils'
  import { MangaDetailStore, CHAPTERS_PREVIEW, type FilterMode } from '../lib/mangaDetail.svelte'
  import ChapterRow from './detail/ChapterRow.svelte'
  import NoChaptersMessage from './detail/NoChaptersMessage.svelte'
  import SectionHeading from './detail/SectionHeading.svelte'
  import CoverImg from './detail/CoverImg.svelte'
  import ChapterListSkeleton from './detail/ChapterListSkeleton.svelte'
  import ReaderModeCard from './detail/ReaderModeCard.svelte'
  import AutoDownloadCard from './detail/AutoDownloadCard.svelte'
  import ExplicitCard from './detail/ExplicitCard.svelte'
  import DownloadDirCard from './detail/DownloadDirCard.svelte'
  import CoverUrlCard from './detail/CoverUrlCard.svelte'
  import ContentTypeCard from './detail/ContentTypeCard.svelte'

  let params: Record<string, string> = $props()
  const id = $derived(params.id)

  const store = new MangaDetailStore()
  $effect(() => store.load(id))

  let synopsisOpen = $state(false)
  let syncLogOpen = $state(false)
  let removeMenuRef: HTMLDivElement | undefined = $state()

  $effect(() => {
    if (!store.showRemoveMenu) return
    function onMouseDown(e: MouseEvent) {
      if (removeMenuRef && !removeMenuRef.contains(e.target as Node)) store.showRemoveMenu = false
    }
    document.addEventListener('mousedown', onMouseDown)
    return () => document.removeEventListener('mousedown', onMouseDown)
  })

  async function removeFromLibrary() {
    await api.removeTitle(id, false)
    router.navigate(ROUTES.library)
  }

  async function removeAndDeleteFiles() {
    if (!window.confirm('Delete all downloaded files for this manga? This cannot be undone.')) return
    await api.removeTitle(id, true)
    router.navigate(ROUTES.library)
  }

  const statsCards = $derived([
    { label: 'Total', value: store.total > 0 ? `${store.total} ch` : '—' },
    { label: 'Downloaded', value: store.downloaded > 0 ? `${store.downloaded} ch` : '—', highlight: store.downloaded > 0 },
    { label: 'Source', value: store.manga ? (store.manga.is_local ? 'local' : 'remote') : '—' },
    { label: 'Year', value: store.manga?.year ? String(store.manga.year) : '—' },
  ])
</script>

<div class="flex flex-col h-full overflow-auto">
  {#if store.loadingManga}
    <div class="h-64 bg-muted animate-pulse shrink-0"></div>
  {:else if store.manga}
    {@const manga = store.manga}
    <div class="relative shrink-0" style="min-height: 272px">
      <div class="absolute inset-0 overflow-hidden">
        <div
          class="absolute inset-0 scale-110"
          style={`background-image: url(${store.coverSrc}); background-size: cover; background-position: center top; filter: blur(28px); opacity: 0.35`}
        ></div>
        <div class="absolute inset-0 bg-gradient-to-b from-background/50 via-background/75 to-background"></div>
      </div>

      <div class="relative z-10 flex items-end gap-4 px-4 md:px-6 pt-10 pb-6 max-w-5xl mx-auto">
        <CoverImg coverUrl={manga.cover_url} mangaId={manga.id} />

        <div class="flex-1 min-w-0 pb-1 space-y-2.5">
          <div class="flex gap-1.5 flex-wrap">
            <ContentTypePill type={manga.content_type} />
            {#if manga.is_explicit}
              <span class="inline-flex items-center px-1.5 py-px rounded text-[10px] font-bold bg-red-500/15 text-red-400">18+</span>
            {/if}
            {#each store.tags.slice(0, 5) as tag (tag)}
              <span class="text-[11px] px-2 py-0.5 rounded-full bg-white/10 border border-white/10 text-foreground/80 font-medium">{tag}</span>
            {/each}
          </div>

          <h1 class="text-3xl font-bold tracking-tight leading-none">{manga.title}</h1>

          <div class="flex items-center gap-2 text-sm text-muted-foreground flex-wrap">
            {#if manga.author}
              <span class="font-medium text-foreground/90">{manga.author}</span>
              <span class="opacity-40">•</span>
            {/if}
            <span class="capitalize">{manga.status}</span>
            {#if store.isSyncing}
              <span class="opacity-40">•</span>
              <span class="flex items-center gap-1 text-primary text-xs">
                <LoaderCircle class="w-3 h-3 animate-spin" /> Syncing…
              </span>
            {/if}
          </div>

          <div class="flex items-center gap-2 pt-0.5 flex-wrap">
            {#if store.resumeChapter}
              {@const resumeChapter = store.resumeChapter}
              <Button
                onclick={() => store.openOrQueue(resumeChapter)}
                class="gap-2"
                disabled={store.pendingReadId === resumeChapter.id && !resumeChapter.downloaded}
              >
                {#if store.pendingReadId === resumeChapter.id && !resumeChapter.downloaded}
                  <LoaderCircle class="w-3.5 h-3.5 animate-spin" />
                {:else}
                  <Download class="w-3.5 h-3.5 fill-current" />
                {/if}
                {store.readCount > 0 ? `Resume Ch. ${resumeChapter.number}` : `Start Ch. ${resumeChapter.number}`}
              </Button>
            {/if}
            {#if store.isRemoteSource && store.streamable > 0}
              <Button
                variant="secondary"
                onclick={() => store.downloadAll()}
                disabled={store.downloadAllPending || store.streamable <= store.activeCount + store.pendingCount}
                class="gap-2"
              >
                <Download class="w-3.5 h-3.5" />
                Download All
              </Button>
            {/if}
            {#if store.isRemoteSource}
              <Button variant="outline" size="icon" onclick={() => store.sync()} disabled={store.syncPending || store.isSyncing} title="Sync chapters">
                <RefreshCw class={cn('w-4 h-4', (store.syncPending || store.isSyncing) && 'animate-spin')} />
              </Button>
            {/if}
            {#if manga.has_sync_warnings}
              <Button
                size="icon"
                onclick={() => store.refreshMetadata()}
                disabled={store.refreshMetaPending}
                title="Source match failed — click to refresh metadata and retry"
                class="bg-amber-500 hover:bg-amber-600 text-white border-0"
              >
                {#if store.refreshMetaPending}
                  <LoaderCircle class="w-4 h-4 animate-spin" />
                {:else}
                  <AlertTriangle class="w-4 h-4" />
                {/if}
              </Button>
            {/if}
            <div class="relative" bind:this={removeMenuRef}>
              <Button variant="outline" size="icon" onclick={() => (store.showRemoveMenu = !store.showRemoveMenu)} title="Remove">
                <Trash2 class="w-4 h-4 text-destructive" />
              </Button>
              {#if store.showRemoveMenu}
                <div class="absolute right-0 top-full mt-1 w-56 rounded-lg border border-border bg-card shadow-lg z-50 overflow-hidden">
                  <button class="w-full text-left px-4 py-2.5 text-sm hover:bg-accent/60 transition-colors" onclick={removeFromLibrary}>
                    Remove from library
                  </button>
                  {#if isAdmin()}
                    <button class="w-full text-left px-4 py-2.5 text-sm text-destructive hover:bg-destructive/10 transition-colors" onclick={removeAndDeleteFiles}>
                      Remove + delete files
                    </button>
                  {/if}
                </div>
              {/if}
            </div>
          </div>
        </div>
      </div>
    </div>
  {/if}

  <div class="max-w-5xl w-full mx-auto px-4 md:px-6 py-6">
    <div class="grid grid-cols-1 md:grid-cols-[1fr_272px] gap-6 items-start">
      <div class="space-y-6 min-w-0 order-2 md:order-1">
        {#if store.manga?.description}
          <section>
            <button class="flex items-center gap-1.5 w-full text-left group" onclick={() => (synopsisOpen = !synopsisOpen)}>
              <SectionHeading>Synopsis</SectionHeading>
              <ChevronDown class={cn('w-3.5 h-3.5 text-muted-foreground transition-transform shrink-0', synopsisOpen && 'rotate-180')} />
            </button>
            {#if synopsisOpen}
              <div class="rounded-lg bg-card border border-border p-4 mt-3">
                <p class="text-sm text-muted-foreground leading-relaxed">{store.manga.description}</p>
              </div>
            {/if}
          </section>
        {/if}

        {#if store.syncLog.length > 0}
          <section>
            <button class="flex items-center gap-1.5 w-full text-left" onclick={() => (syncLogOpen = !syncLogOpen)}>
              <SectionHeading>
                <Terminal class="w-3.5 h-3.5 inline-block mr-1.5 opacity-60" />
                Last Sync
                {#if store.isSyncing}<LoaderCircle class="w-3 h-3 animate-spin inline-block ml-2 text-primary" />{/if}
              </SectionHeading>
              <ChevronDown class={cn('w-3.5 h-3.5 text-muted-foreground transition-transform shrink-0', syncLogOpen && 'rotate-180')} />
            </button>
            {#if syncLogOpen}
              <div class="mt-3 rounded-lg bg-[#0d0f10] border border-border overflow-hidden">
                <div class="max-h-52 overflow-y-auto px-3 py-2.5 space-y-1">
                  {#each store.syncLog as entry (entry.id)}
                    <div class="flex items-start gap-2 text-xs font-mono">
                      <span class="text-muted-foreground/40 shrink-0 text-[10px] mt-px">
                        {new Date(entry.created_at).toLocaleTimeString([], { hour: '2-digit', minute: '2-digit', second: '2-digit' })}
                      </span>
                      <span class="text-foreground/80 break-all">{entry.message}</span>
                    </div>
                  {/each}
                </div>
              </div>
            {/if}
          </section>
        {/if}

        <section>
          <div class="flex items-center justify-between gap-3 flex-wrap">
            <SectionHeading>
              Chapters
              {#if store.total > 0}
                <span class="ml-1.5 text-xs font-normal normal-case tracking-normal text-muted-foreground/50">
                  {store.filteredChapters.length !== store.total ? `${store.filteredChapters.length} / ${store.total}` : `(${store.total})`}
                </span>
              {/if}
            </SectionHeading>

            {#if store.total > 0}
              <div class="flex items-center gap-2">
                <div class="flex items-center gap-1 bg-muted rounded-lg p-0.5">
                  {#each ['all', 'downloaded', 'not_downloaded'] as mode (mode)}
                    <button
                      onclick={() => {
                        store.filterMode = mode as FilterMode
                        store.showAll = false
                      }}
                      class={cn(
                        'text-[11px] font-medium px-2.5 py-1 rounded-md transition-colors capitalize',
                        store.filterMode === mode ? 'bg-card text-foreground shadow-sm' : 'text-muted-foreground hover:text-foreground',
                      )}
                    >
                      {mode === 'not_downloaded' ? 'Not Downloaded' : mode === 'downloaded' ? 'Downloaded' : 'All'}
                    </button>
                  {/each}
                </div>
                <button
                  onclick={() => (store.sortDir = store.sortDir === 'desc' ? 'asc' : 'desc')}
                  class="flex items-center gap-1 px-2 py-1.5 rounded-lg bg-muted hover:bg-accent text-muted-foreground hover:text-foreground transition-colors text-[11px] font-medium"
                >
                  {#if store.sortDir === 'desc'}<ArrowDown class="w-3.5 h-3.5" />{:else}<ArrowUp class="w-3.5 h-3.5" />{/if}
                  {store.sortDir === 'desc' ? 'Newest' : 'Oldest'}
                </button>
              </div>
            {/if}
          </div>

          {#if store.activeCount > 0 || store.pendingCount > 0}
            <div class="flex items-center justify-between mt-3 px-3 py-2 rounded-lg bg-primary/10 border border-primary/20">
              <div class="flex items-center gap-2 text-xs text-primary">
                <LoaderCircle class="w-3.5 h-3.5 animate-spin shrink-0" />
                <span>
                  {#if store.activeCount > 0}{store.activeCount} downloading{/if}
                  {#if store.activeCount > 0 && store.pendingCount > 0}&nbsp;·{/if}
                  {#if store.pendingCount > 0}{store.pendingCount} queued{/if}
                </span>
              </div>
              <button
                onclick={() => store.cancelAll()}
                disabled={store.cancelAllPending}
                class="flex items-center gap-1 text-[11px] font-medium text-primary/70 hover:text-destructive transition-colors"
              >
                <X class="w-3 h-3" />
                Cancel all
              </button>
            </div>
          {/if}

          <div class="mt-3">
            {#if store.loadingChapters}
              <ChapterListSkeleton />
            {:else if store.total === 0}
              <NoChaptersMessage
                hasSyncWarnings={store.manga?.has_sync_warnings ?? false}
                isSyncing={store.isSyncing}
                isPending={store.syncPending}
                isRemoteSource={store.isRemoteSource}
                onSync={() => store.sync()}
              />
            {:else}
              <div class="space-y-1.5">
                {#each store.displayed as ch (ch.id)}
                  <ChapterRow
                    chapter={ch}
                    progress={store.progressMap.get(ch.id) ?? null}
                    queueItem={store.queueMap.get(ch.id) ?? null}
                    pendingRead={store.pendingReadId === ch.id}
                    onOpen={() => store.openOrQueue(ch)}
                    onCancelDownload={(qid) => store.removeFromQueue(qid)}
                  />
                {/each}
              </div>
              {#if store.filteredChapters.length > CHAPTERS_PREVIEW}
                <button
                  onclick={() => (store.showAll = !store.showAll)}
                  class="mt-3 w-full text-sm text-muted-foreground hover:text-foreground transition-colors py-2 flex items-center justify-center gap-1.5"
                >
                  {store.showAll ? 'Show less' : `View all ${store.filteredChapters.length} chapters`}
                  <ChevronDown class={cn('w-4 h-4 transition-transform', store.showAll && 'rotate-180')} />
                </button>
              {/if}
            {/if}
          </div>
        </section>
      </div>

      <div class="space-y-4 order-1 md:order-2">
        {#if store.total > 0}
          <div class="rounded-lg bg-card border border-border p-4 space-y-3">
            <div class="flex items-center justify-between">
              <span class="text-[10px] font-semibold uppercase tracking-wider text-muted-foreground">Reading Progress</span>
              <span class="text-sm font-bold">{store.readPct}%</span>
            </div>
            <div class="h-1.5 rounded-full bg-muted overflow-hidden">
              <div class="h-full bg-primary rounded-full transition-all" style={`width: ${store.readPct}%`}></div>
            </div>
            <p class="text-[11px] text-muted-foreground">Chapter {store.readCount} of {store.total} completed</p>
          </div>
        {/if}

        <div class="rounded-lg bg-card border border-border overflow-hidden">
          <div class="grid grid-cols-2 divide-x divide-y divide-border">
            {#each statsCards as { label, value, highlight } (label)}
              <div class="p-3">
                <p class="text-[10px] text-muted-foreground uppercase tracking-wider">{label}</p>
                <p class={cn('text-sm font-semibold mt-0.5 capitalize', highlight && 'text-[#34d399]')}>{value}</p>
              </div>
            {/each}
            {#if store.activeCount > 0 || store.pendingCount > 0}
              <div class="p-3">
                <p class="text-[10px] text-muted-foreground uppercase tracking-wider">Downloading</p>
                <p class="text-sm font-semibold mt-0.5 text-primary flex items-center gap-1">
                  <LoaderCircle class="w-3 h-3 animate-spin shrink-0" />{store.activeCount} ch
                </p>
              </div>
              <div class="p-3">
                <p class="text-[10px] text-muted-foreground uppercase tracking-wider">Queued</p>
                <p class="text-sm font-semibold mt-0.5 text-muted-foreground flex items-center gap-1">
                  <Download class="w-3 h-3 shrink-0" />{store.pendingCount} ch
                </p>
              </div>
            {/if}
          </div>
        </div>

        {#if store.manga && !store.manga.is_local}
          <AutoDownloadCard mangaId={store.manga.id} value={store.manga.auto_download} />
        {/if}
        {#if store.manga}
          <ReaderModeCard mangaId={store.manga.id} value={store.manga.reader_mode ?? null} />
        {/if}
        {#if store.manga}
          <DownloadDirCard mangaId={store.manga.id} value={store.manga.download_dir ?? null} />
        {/if}
        {#if store.manga && isAdmin()}
          <ExplicitCard mangaId={store.manga.id} value={store.manga.is_explicit ?? false} />
        {/if}
        {#if store.manga && isAdmin()}
          <ContentTypeCard mangaId={store.manga.id} value={store.manga.content_type} />
        {/if}
        {#if store.manga && isAdmin()}
          <CoverUrlCard
            mangaId={store.manga.id}
            value={store.manga.cover_url?.startsWith('http') ? store.manga.cover_url : null}
            onSaved={() => store.refreshManga()}
          />
        {/if}

        {#if store.manga?.author}
          <div class="rounded-lg bg-card border border-border p-4">
            <p class="text-[10px] font-semibold uppercase tracking-wider text-muted-foreground mb-1.5">Author</p>
            <p class="text-sm font-medium">{store.manga.author}</p>
          </div>
        {/if}
      </div>
    </div>
  </div>
</div>
