<script lang="ts">
  import { LoaderCircle, Trash2 } from '@lucide/svelte'
  import { api } from '../../lib/api'
  import type { Title } from '../../lib/types'
  import { cn } from '../../lib/utils'
  import ContentTypePill from '../../lib/components/ContentTypePill.svelte'

  let {
    manga,
    onClick,
    onRemove,
    isRemoving,
    syncMessage,
  }: {
    manga: Title
    onClick: () => void
    onRemove: (deleteFiles: boolean) => void
    isRemoving: boolean
    syncMessage?: string
  } = $props()

  let imgFailed = $state(false)
  let confirming = $state(false)

  const src = $derived(
    !imgFailed && (manga.cover_url?.startsWith('http') || manga.cover_url?.startsWith('/api/'))
      ? (manga.cover_url as string)
      : api.coverUrl(manga.id),
  )
  const isSyncing = $derived(manga.sync_status === 'syncing')
  const hasWarnings = $derived(manga.has_sync_warnings)
  const hasDownloads = $derived((manga.downloaded_chapters ?? 0) > 0)

  function handleKeydown(e: KeyboardEvent) {
    if (e.key === 'Enter' || e.key === ' ') onClick()
  }
</script>

<div
  data-nav
  role="button"
  tabindex="0"
  class="group relative cursor-pointer"
  onclick={onClick}
  onkeydown={handleKeydown}
>
  <div class="relative rounded-xl overflow-hidden transition-all duration-300 ease-out group-hover:scale-[1.04] group-hover:shadow-[0_0_20px_rgba(139,92,246,0.35)]">
    {#if imgFailed}
      <div class="w-full aspect-[2/3] bg-muted flex items-center justify-center text-4xl rounded-xl">📖</div>
    {:else}
      <img
        src={src}
        alt={manga.title}
        class={cn('w-full aspect-[2/3] object-cover bg-muted block', isSyncing && 'opacity-40')}
        onerror={() => (imgFailed = true)}
      />
    {/if}
    <div class="absolute inset-x-0 bottom-0 h-1/3 bg-gradient-to-t from-black/70 to-transparent opacity-0 group-hover:opacity-100 transition-opacity duration-300 pointer-events-none"></div>
    {#if isSyncing}
      <div class="absolute inset-0 flex flex-col items-center justify-center gap-1.5 px-2 pointer-events-none">
        <LoaderCircle class="w-5 h-5 animate-spin text-primary shrink-0" />
        <span class="text-[10px] font-medium text-primary text-center line-clamp-3 leading-tight">
          {syncMessage ?? 'Building…'}
        </span>
      </div>
    {/if}
    {#if hasWarnings && !isSyncing}
      <div
        class="absolute top-2 left-2 w-5 h-5 rounded-full bg-amber-500/90 flex items-center justify-center"
        title="Some sources could not be matched — click title to refresh metadata"
      >
        <span class="text-[9px] font-bold text-white leading-none">!</span>
      </div>
    {/if}
    {#if !confirming}
      <button
        class="absolute top-2 right-2 w-7 h-7 rounded-lg bg-black/60 text-white flex items-center justify-center opacity-0 group-hover:opacity-100 focus:opacity-100 hover:bg-destructive hover:text-destructive-foreground transition-all"
        onclick={(e) => {
          e.stopPropagation()
          confirming = true
        }}
        disabled={isRemoving}
      >
        {#if isRemoving}
          <LoaderCircle class="w-3 h-3 animate-spin" />
        {:else}
          <Trash2 class="w-3 h-3" />
        {/if}
      </button>
    {:else}
      <div
        class="absolute inset-x-2 top-2 bg-card/95 backdrop-blur rounded-lg p-2 flex flex-col gap-1.5 shadow-lg"
        onclick={(e) => e.stopPropagation()}
        role="presentation"
      >
        <p class="text-[10px] font-semibold text-center text-foreground">Remove title?</p>
        {#if hasDownloads}
          <button
            class="w-full text-[10px] px-2 py-1 rounded bg-destructive text-destructive-foreground font-medium hover:opacity-90 transition-opacity"
            onclick={() => {
              confirming = false
              onRemove(true)
            }}
          >
            Remove + delete files
          </button>
        {/if}
        <button
          class="w-full text-[10px] px-2 py-1 rounded bg-muted text-foreground font-medium hover:bg-accent transition-colors"
          onclick={() => {
            confirming = false
            onRemove(false)
          }}
        >
          {hasDownloads ? 'Library only' : 'Remove'}
        </button>
        <button
          class="w-full text-[10px] text-muted-foreground hover:text-foreground transition-colors"
          onclick={() => (confirming = false)}
        >
          Cancel
        </button>
      </div>
    {/if}
  </div>

  <div class="mt-2.5 px-0.5 space-y-1">
    <p class="text-sm font-semibold leading-snug line-clamp-2">{manga.title}</p>
    <div class="flex gap-1 flex-wrap items-center">
      <ContentTypePill type={manga.content_type} size="sm" />
      {#if manga.is_explicit}
        <span class="inline-flex items-center px-1.5 py-px rounded text-[10px] font-bold bg-red-500/15 text-red-400">18+</span>
      {/if}
    </div>
    {#if manga.author}
      <p class="text-[11px] text-muted-foreground truncate">{manga.author}</p>
    {/if}
    {#if (manga.total_chapters ?? 0) > 0}
      <div class="space-y-1">
        <div class="flex items-center justify-between text-[10px] text-muted-foreground">
          <span>{manga.total_chapters} ch{(manga.downloaded_chapters ?? 0) > 0 ? ` · ${manga.downloaded_chapters} DL` : ''}</span>
          <span>{manga.chapters_read ?? 0}/{manga.total_chapters} read</span>
        </div>
        <div class="h-0.5 rounded-full bg-muted overflow-hidden">
          <div
            class="h-full bg-primary/70 rounded-full transition-all"
            style={`width: ${Math.round(((manga.chapters_read ?? 0) / (manga.total_chapters ?? 1)) * 100)}%`}
          ></div>
        </div>
      </div>
    {/if}
  </div>
</div>
