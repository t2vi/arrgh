import { Download, HardDrive, CheckCircle2, BookOpen, Clock, Loader2, AlertCircle, X } from 'lucide-react'
import { type QueueItem } from '@/api'
import type { Chapter, ReadProgress } from '@/types'
import { cn } from '@/lib/utils'

function formatDate(iso: string) {
  return new Date(iso).toLocaleDateString('en-US', { month: 'short', day: 'numeric', year: 'numeric' })
}

export function ChapterRow({
  chapter, progress, queueItem, pendingRead, onOpen, onCancelDownload,
}: {
  chapter: Chapter
  progress: ReadProgress | null
  queueItem: QueueItem | null
  pendingRead: boolean
  onOpen: () => void
  onCancelDownload: (queueId: string) => void
}) {
  const isCompleted  = progress?.completed === true
  const isStarted    = progress != null && !isCompleted
  const pct          = chapter.page_count > 0 && progress
    ? Math.round((progress.current_page / chapter.page_count) * 100)
    : 0
  const isDownloaded = chapter.downloaded || queueItem?.status === 'done'
  const isPending    = !isDownloaded && queueItem?.status === 'pending'
  const isActive     = !isDownloaded && queueItem?.status === 'downloading'
  const isError      = !isDownloaded && queueItem?.status === 'error'
  const isWaiting    = pendingRead && !isDownloaded
  const isClickable  = isDownloaded || (chapter.has_sources && !isError)

  return (
    <div
      className={cn(
        'flex items-center gap-3 px-3 py-2.5 rounded-lg border border-border bg-card transition-colors',
        isClickable && 'cursor-pointer hover:bg-accent/40',
        isCompleted && 'opacity-50',
      )}
      onClick={isClickable ? onOpen : undefined}
    >
      <span className={cn(
        'w-11 text-center text-xs font-bold py-1.5 rounded-md shrink-0 leading-none',
        isDownloaded ? 'bg-primary/20 text-primary' : 'bg-muted text-muted-foreground',
      )}>
        {chapter.number}
      </span>

      <div className="flex-1 min-w-0">
        <p className="text-sm font-medium truncate">
          {chapter.title ?? `Chapter ${chapter.number}`}
        </p>
        <div className="flex items-center gap-1.5 text-[11px] text-muted-foreground mt-0.5">
          <span>{formatDate(chapter.created_at)}</span>
          {chapter.page_count > 0 && (
            <>
              <span className="opacity-40">•</span>
              <span>{chapter.page_count} Pages</span>
            </>
          )}
          {isStarted && (
            <>
              <span className="opacity-40">•</span>
              <span className="text-primary">{progress!.current_page + 1}/{chapter.page_count}</span>
            </>
          )}
          {isWaiting && (
            <>
              <span className="opacity-40">•</span>
              <span className="text-primary">Opening when ready…</span>
            </>
          )}
        </div>
        {isStarted && (
          <div className="w-full h-0.5 rounded-full bg-muted mt-1.5">
            <div className="h-full rounded-full bg-primary transition-all" style={{ width: `${pct}%` }} />
          </div>
        )}
        {isActive && queueItem && queueItem.pages_total > 0 && (
          <div className="flex items-center gap-2 mt-1.5">
            <div className="flex-1 h-0.5 rounded-full bg-muted overflow-hidden">
              <div
                className="h-full bg-primary transition-all duration-300"
                style={{ width: `${Math.round((queueItem.pages_downloaded / queueItem.pages_total) * 100)}%` }}
              />
            </div>
            <span className="text-[10px] text-muted-foreground tabular-nums shrink-0">
              {Math.round((queueItem.pages_downloaded / queueItem.pages_total) * 100)}%
            </span>
          </div>
        )}
      </div>

      <div className="flex items-center gap-1 shrink-0" onClick={(e) => e.stopPropagation()}>
        {isCompleted && <CheckCircle2 className="w-3.5 h-3.5 text-muted-foreground mr-0.5" />}
        {isDownloaded ? (
          <button className="w-7 h-7 flex items-center justify-center rounded-md hover:bg-accent transition-colors" onClick={onOpen} title="Read">
            <BookOpen className="w-3.5 h-3.5 text-primary" />
          </button>
        ) : isActive || isWaiting ? (
          <Loader2 className="w-4 h-4 text-primary animate-spin" />
        ) : isPending ? (
          <button
            onClick={() => onCancelDownload(queueItem!.id)}
            className="flex items-center gap-1 px-2 py-1 rounded-md text-[11px] font-medium bg-muted text-muted-foreground hover:bg-destructive/10 hover:text-destructive transition-colors"
          >
            <Clock className="w-3 h-3 shrink-0" />
            Queued
            <X className="w-3 h-3 shrink-0" />
          </button>
        ) : isError ? (
          <button
            className="w-7 h-7 flex items-center justify-center rounded-md hover:bg-accent transition-colors text-destructive"
            onClick={onOpen}
            title="Retry"
          >
            <AlertCircle className="w-3.5 h-3.5" />
          </button>
        ) : chapter.has_sources ? (
          <button
            className="w-7 h-7 flex items-center justify-center rounded-md hover:bg-accent transition-colors"
            onClick={onOpen}
            title="Download & read"
          >
            <Download className="w-3.5 h-3.5 text-muted-foreground" />
          </button>
        ) : (
          <HardDrive className="w-3.5 h-3.5 text-muted-foreground/40" />
        )}
      </div>
    </div>
  )
}
