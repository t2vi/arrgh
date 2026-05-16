import { useState, useEffect, useRef, useCallback } from 'react'
import { useNavigate } from 'react-router-dom'
import { Search, Loader2, Trash2, ChevronDown, SlidersHorizontal, Plus } from 'lucide-react'
import { api } from '@/api'
import type { Manga } from '@/types'
import { Input } from '@/components/ui/input'
import { Button } from '@/components/ui/button'
import { Skeleton } from '@/components/ui/skeleton'
import { cn } from '@/lib/utils'
import { ContentTypePill } from '@/components/ContentTypePill'
import { ROUTES } from '@/lib/routes'

interface PaginatedManga {
  items: Manga[]
  total: number
  limit: number
}

export default function Library() {
  const [search, setSearch] = useState('')
  const [page, setPage] = useState(1)
  const [data, setData] = useState<PaginatedManga | undefined>()
  const [loading, setLoading] = useState(true)
  const [removingId, setRemovingId] = useState<string | null>(null)
  const navigate = useNavigate()
  const intervalRef = useRef<ReturnType<typeof setInterval> | null>(null)

  const fetchData = useCallback(() => {
    api.listManga(page, search || undefined)
      .then(setData)
      .catch(() => {})
      .finally(() => setLoading(false))
  }, [page, search])

  useEffect(() => {
    setLoading(true)
    fetchData()
  }, [fetchData])

  // Polling: every 2s if any manga is syncing
  useEffect(() => {
    if (intervalRef.current) clearInterval(intervalRef.current)
    intervalRef.current = setInterval(() => {
      if (data?.items.some((m) => m.sync_status === 'syncing')) {
        api.listManga(page, search || undefined).then(setData).catch(() => {})
      }
    }, 2000)
    return () => {
      if (intervalRef.current) clearInterval(intervalRef.current)
    }
  }, [data, page, search])

  async function handleRemove(id: string, deleteFiles: boolean) {
    setRemovingId(id)
    try {
      await api.removeManga(id, deleteFiles)
      fetchData()
    } catch {
      // ignore
    } finally {
      setRemovingId(null)
    }
  }

  const totalPages = data ? Math.ceil(data.total / data.limit) : 1

  return (
    <>
      <header className="flex items-center gap-4 px-6 py-3 border-b border-border shrink-0 bg-background/80 backdrop-blur-xl sticky top-0 z-10">
        <div className="flex-1 max-w-sm">
          <div className="relative">
            <Search className="absolute left-3 top-1/2 -translate-y-1/2 w-4 h-4 text-muted-foreground pointer-events-none" />
            <Input
              className="pl-9 rounded-full bg-muted border-transparent focus-visible:border-ring"
              placeholder="Search your library…"
              value={search}
              onChange={(e) => { setSearch(e.target.value); setPage(1) }}
            />
          </div>
        </div>

        <div className="flex-1" />

        <div className="flex items-center gap-2 shrink-0">
          <Button variant="outline" size="sm" className="gap-1.5 text-xs">
            <span className="text-muted-foreground">Sort by:</span>
            Recently Added
            <ChevronDown className="w-3 h-3 text-muted-foreground" />
          </Button>
          <Button variant="outline" size="sm" className="gap-1.5 text-xs">
            <SlidersHorizontal className="w-3 h-3" />
            Filters
          </Button>
        </div>
      </header>

      <div className="flex-1 overflow-auto">
        <div className="p-6 space-y-5">
          <div>
            <h2 className="text-2xl font-bold tracking-tight">My Library</h2>
            <p className="text-sm text-muted-foreground mt-0.5">
              {data ? `${data.total} saved ${data.total === 1 ? 'title' : 'titles'}` : ' '}
            </p>
          </div>

          {loading && <MangaGridSkeleton />}

          {data && (
            <>
              {data.items.length === 0 && (
                <p className="text-muted-foreground text-sm py-16 text-center">
                  {search ? 'No results.' : 'Library is empty — discover manga to add some.'}
                </p>
              )}

              <div className="grid grid-cols-[repeat(auto-fill,minmax(160px,1fr))] gap-5">
                {data.items.map((m) => (
                  <MangaCard
                    key={m.id}
                    manga={m}
                    onClick={() => navigate(ROUTES.manga(m.id))}
                    onRemove={(deleteFiles) => handleRemove(m.id, deleteFiles)}
                    isRemoving={removingId === m.id}
                  />
                ))}
              </div>

              {totalPages > 1 && (
                <div className="flex items-center justify-center gap-3 pt-2">
                  <Button variant="outline" size="sm" disabled={page === 1} onClick={() => setPage((p) => p - 1)}>
                    Prev
                  </Button>
                  <span className="text-sm text-muted-foreground">{page} / {totalPages}</span>
                  <Button variant="outline" size="sm" disabled={page >= totalPages} onClick={() => setPage((p) => p + 1)}>
                    Next
                  </Button>
                </div>
              )}
            </>
          )}
        </div>
      </div>

      <button
        onClick={() => navigate(ROUTES.discover)}
        title="Discover manga"
        className="fixed bottom-6 right-6 w-12 h-12 rounded-full bg-primary text-primary-foreground flex items-center justify-center shadow-xl hover:opacity-90 transition-opacity z-10"
      >
        <Plus className="w-5 h-5" />
      </button>
    </>
  )
}

function MangaCard({
  manga, onClick, onRemove, isRemoving,
}: {
  manga: Manga
  onClick: () => void
  onRemove: (deleteFiles: boolean) => void
  isRemoving: boolean
}) {
  const [imgFailed, setImgFailed] = useState(false)
  const [confirming, setConfirming] = useState(false)
  const src = !imgFailed && manga.cover_url?.startsWith('http') ? manga.cover_url : api.coverUrl(manga.id)
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

function MangaGridSkeleton() {
  return (
    <div className="grid grid-cols-[repeat(auto-fill,minmax(160px,1fr))] gap-5">
      {Array.from({ length: 12 }).map((_, i) => (
        <div key={i}>
          <Skeleton className="w-full aspect-[2/3] rounded-xl" />
          <Skeleton className="h-3.5 mt-2.5 w-3/4 rounded" />
          <Skeleton className="h-3 mt-1.5 w-1/2 rounded" />
        </div>
      ))}
    </div>
  )
}
