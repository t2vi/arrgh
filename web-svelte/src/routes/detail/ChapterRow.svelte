<script lang="ts">
  import { Download, HardDrive, CheckCircle2, BookOpen, Clock, LoaderCircle, AlertCircle, X } from '@lucide/svelte'
  import type { QueueItem } from '../../lib/api'
  import type { Chapter, ReadProgress } from '../../lib/types'
  import { cn } from '../../lib/utils'

  function formatDate(iso: string) {
    return new Date(iso).toLocaleDateString('en-US', { month: 'short', day: 'numeric', year: 'numeric' })
  }

  let {
    chapter,
    progress,
    queueItem,
    pendingRead,
    onOpen,
    onCancelDownload,
  }: {
    chapter: Chapter
    progress: ReadProgress | null
    queueItem: QueueItem | null
    pendingRead: boolean
    onOpen: () => void
    onCancelDownload: (queueId: string) => void
  } = $props()

  const isCompleted = $derived(progress?.completed === true)
  const isStarted = $derived(progress != null && !isCompleted)
  const pct = $derived(chapter.page_count > 0 && progress ? Math.round((progress.current_page / chapter.page_count) * 100) : 0)
  const isDownloaded = $derived(chapter.downloaded || queueItem?.status === 'done')
  const isPending = $derived(!isDownloaded && queueItem?.status === 'pending')
  const isActive = $derived(!isDownloaded && queueItem?.status === 'downloading')
  const isError = $derived(!isDownloaded && queueItem?.status === 'error')
  const isWaiting = $derived(pendingRead && !isDownloaded)
  const isClickable = $derived(isDownloaded || (chapter.has_sources && !isError))
</script>

<!-- svelte-ignore a11y_no_noninteractive_tabindex -- role is 'button' whenever tabindex is 0 -->
<div
  data-chapter-id={chapter.id}
  class={cn(
    'flex items-center gap-3 px-3 py-2.5 rounded-lg border border-border bg-card transition-colors',
    isClickable && 'cursor-pointer hover:bg-accent/40',
    isCompleted && 'opacity-50',
  )}
  onclick={isClickable ? onOpen : undefined}
  role={isClickable ? 'button' : undefined}
  tabindex={isClickable ? 0 : undefined}
  onkeydown={isClickable ? (e: KeyboardEvent) => (e.key === 'Enter' || e.key === ' ') && onOpen() : undefined}
>
  <span
    class={cn(
      'w-11 text-center text-xs font-bold py-1.5 rounded-md shrink-0 leading-none',
      isDownloaded ? 'bg-primary/20 text-primary' : 'bg-muted text-muted-foreground',
    )}
  >
    {chapter.number}
  </span>

  <div class="flex-1 min-w-0">
    <p class="text-sm font-medium truncate">{chapter.title ?? `Chapter ${chapter.number}`}</p>
    <div class="flex items-center gap-1.5 text-[11px] text-muted-foreground mt-0.5">
      <span>{formatDate(chapter.created_at)}</span>
      {#if chapter.page_count > 0}
        <span class="opacity-40">•</span>
        <span>{chapter.page_count} pages</span>
      {/if}
      {#if isStarted}
        <span class="opacity-40">•</span>
        <span class="text-primary">{progress!.current_page + 1}/{chapter.page_count}</span>
      {/if}
      {#if isWaiting}
        <span class="opacity-40">•</span>
        <span class="text-primary">Opening when ready…</span>
      {/if}
    </div>
    {#if isStarted}
      <div class="w-full h-0.5 rounded-full bg-muted mt-1.5">
        <div class="h-full rounded-full bg-primary transition-all" style={`width: ${pct}%`}></div>
      </div>
    {/if}
    {#if isActive && queueItem && queueItem.pages_total > 0}
      <div class="flex items-center gap-2 mt-1.5">
        <div class="flex-1 h-0.5 rounded-full bg-muted overflow-hidden">
          <div
            class="h-full bg-primary transition-all duration-300"
            style={`width: ${Math.round((queueItem.pages_downloaded / queueItem.pages_total) * 100)}%`}
          ></div>
        </div>
        <span class="text-[10px] text-muted-foreground tabular-nums shrink-0">
          {Math.round((queueItem.pages_downloaded / queueItem.pages_total) * 100)}%
        </span>
      </div>
    {/if}
  </div>

  <div class="flex items-center gap-1 shrink-0" onclick={(e) => e.stopPropagation()} role="presentation">
    {#if isCompleted}
      <CheckCircle2 class="w-3.5 h-3.5 text-muted-foreground mr-0.5" />
    {/if}
    {#if isDownloaded}
      <button class="w-7 h-7 flex items-center justify-center rounded-md hover:bg-accent transition-colors" onclick={onOpen} title="Read">
        <BookOpen class="w-3.5 h-3.5 text-primary" />
      </button>
    {:else if isActive || isWaiting}
      <LoaderCircle class="w-4 h-4 text-primary animate-spin" />
    {:else if isPending}
      <button
        onclick={() => onCancelDownload(queueItem!.id)}
        class="flex items-center gap-1 px-2 py-1 rounded-md text-[11px] font-medium bg-muted text-muted-foreground hover:bg-destructive/10 hover:text-destructive transition-colors"
      >
        <Clock class="w-3 h-3 shrink-0" />
        Queued
        <X class="w-3 h-3 shrink-0" />
      </button>
    {:else if isError}
      <button class="w-7 h-7 flex items-center justify-center rounded-md hover:bg-accent transition-colors text-destructive" onclick={onOpen} title="Retry">
        <AlertCircle class="w-3.5 h-3.5" />
      </button>
    {:else if chapter.has_sources}
      <button class="w-7 h-7 flex items-center justify-center rounded-md hover:bg-accent transition-colors" onclick={onOpen} title="Download & read">
        <Download class="w-3.5 h-3.5 text-muted-foreground" />
      </button>
    {:else}
      <HardDrive class="w-3.5 h-3.5 text-muted-foreground/40" />
    {/if}
  </div>
</div>
