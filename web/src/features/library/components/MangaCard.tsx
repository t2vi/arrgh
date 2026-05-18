import { useState } from 'react'
import { Loader2, Trash2 } from 'lucide-react'
import { api } from '@/api'
import type { Manga } from '@/types'
import { cn } from '@/lib/utils'
import { ContentTypePill } from '@/components/ContentTypePill'

export function MangaCard({
  manga, onClick, onRemove, isRemoving,
}: {
  manga: Manga
  onClick: () => void
  onRemove: (deleteFiles: boolean) => void
  isRemoving: boolean
}) {
  const [imgFailed, setImgFailed] = useState(false)
  const [confirming, setConfirming] = useState(false)
  const src = !imgFailed && (manga.cover_url?.startsWith('http') || manga.cover_url?.startsWith('/api/'))
    ? manga.cover_url
    : api.coverUrl(manga.id)
  const isSyncing    = manga.sync_status === 'syncing'
  const hasDownloads = (manga.downloaded_chapters ?? 0) > 0

  return (
    <div
      data-nav
      tabIndex={isSyncing ? undefined : 0}
      className={cn('group relative', !isSyncing && 'cursor-pointer')}
      onClick={isSyncing ? undefined : onClick}
      onKeyDown={(e) => { if (!isSyncing && (e.key === 'Enter' || e.key === ' ')) onClick() }}
    >
      <div className={cn(
        'relative rounded-xl overflow-hidden transition-all duration-300 ease-out',
        !isSyncing && 'group-hover:scale-[1.04] group-hover:shadow-[0_0_20px_rgba(139,92,246,0.35)]',
      )}>
        {imgFailed ? (
          <div className="w-full aspect-[2/3] bg-muted flex items-center justify-center text-4xl rounded-xl">📖</div>
        ) : (
          <img
            src={src}
            alt={manga.title}
            className={cn('w-full aspect-[2/3] object-cover bg-muted block', isSyncing && 'opacity-40')}
            onError={() => setImgFailed(true)}
          />
        )}
        {!isSyncing && (
          <div className="absolute inset-x-0 bottom-0 h-1/3 bg-gradient-to-t from-black/70 to-transparent opacity-0 group-hover:opacity-100 transition-opacity duration-300 pointer-events-none" />
        )}
        {isSyncing && (
          <div className="absolute inset-0 flex flex-col items-center justify-center gap-1.5 pointer-events-none">
            <Loader2 className="w-5 h-5 animate-spin text-primary" />
            <span className="text-[10px] font-medium text-primary">Building…</span>
          </div>
        )}
        {!confirming ? (
          <button
            className="absolute top-2 right-2 w-7 h-7 rounded-lg bg-black/60 text-white flex items-center justify-center opacity-0 group-hover:opacity-100 focus:opacity-100 hover:bg-destructive hover:text-destructive-foreground transition-all"
            onClick={(e) => { e.stopPropagation(); setConfirming(true) }}
            disabled={isRemoving}
          >
            {isRemoving ? <Loader2 className="w-3 h-3 animate-spin" /> : <Trash2 className="w-3 h-3" />}
          </button>
        ) : (
          <div
            className="absolute inset-x-2 top-2 bg-card/95 backdrop-blur rounded-lg p-2 flex flex-col gap-1.5 shadow-lg"
            onClick={(e) => e.stopPropagation()}
          >
            <p className="text-[10px] font-semibold text-center text-foreground">Remove manga?</p>
            {hasDownloads && (
              <button
                className="w-full text-[10px] px-2 py-1 rounded bg-destructive text-destructive-foreground font-medium hover:opacity-90 transition-opacity"
                onClick={() => { setConfirming(false); onRemove(true) }}
              >
                Remove + delete files
              </button>
            )}
            <button
              className="w-full text-[10px] px-2 py-1 rounded bg-muted text-foreground font-medium hover:bg-accent transition-colors"
              onClick={() => { setConfirming(false); onRemove(false) }}
            >
              {hasDownloads ? 'Library only' : 'Remove'}
            </button>
            <button
              className="w-full text-[10px] text-muted-foreground hover:text-foreground transition-colors"
              onClick={() => setConfirming(false)}
            >
              Cancel
            </button>
          </div>
        )}
      </div>

      <div className="mt-2.5 px-0.5 space-y-1">
        <p className="text-sm font-semibold leading-snug line-clamp-2">{manga.title}</p>
        <ContentTypePill type={manga.content_type} size="sm" />
        {manga.author && (
          <p className="text-[11px] text-muted-foreground truncate">{manga.author}</p>
        )}
        {(manga.total_chapters ?? 0) > 0 && (
          <div className="space-y-1">
            <div className="flex items-center justify-between text-[10px] text-muted-foreground">
              <span>{manga.total_chapters} ch{(manga.downloaded_chapters ?? 0) > 0 && ` · ${manga.downloaded_chapters} DL`}</span>
              <span>{manga.chapters_read ?? 0}/{manga.total_chapters} read</span>
            </div>
            <div className="h-0.5 rounded-full bg-muted overflow-hidden">
              <div
                className="h-full bg-primary/70 rounded-full transition-all"
                style={{ width: `${Math.round(((manga.chapters_read ?? 0) / (manga.total_chapters ?? 1)) * 100)}%` }}
              />
            </div>
          </div>
        )}
      </div>
    </div>
  )
}
