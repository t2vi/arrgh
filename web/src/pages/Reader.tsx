import { useEffect, useRef, useState } from 'react'
import { useParams, useNavigate } from 'react-router-dom'
import { ChevronRight, ChevronLeft, Library, AlignJustify, BookOpen } from 'lucide-react'
import { api } from '@/api'
import { ROUTES } from '@/lib/routes'
import { Button } from '@/components/ui/button'
import { cn } from '@/lib/utils'

export default function Reader() {
  const { chapterId } = useParams<{ chapterId: string }>()
  const navigate = useNavigate()
  const [page, setPage] = useState(0)
  const [lastPage, setLastPage] = useState<number | null>(null)
  // null = follow settings/manga; override set by in-reader toggle
  const [modeOverride, setModeOverride] = useState<'paged' | 'scroll' | null>(null)

  const [chapter, setChapter] = useState<Awaited<ReturnType<typeof api.getChapter>> | undefined>()
  const [progress, setProgress] = useState<Awaited<ReturnType<typeof api.getProgress>> | null>(null)
  const [settings, setSettings] = useState<Awaited<ReturnType<typeof api.getSettings>> | undefined>()
  const [manga, setManga] = useState<Awaited<ReturnType<typeof api.getManga>> | undefined>()

  useEffect(() => {
    if (!chapterId) return
    api.getChapter(chapterId).then(setChapter).catch(() => {})
    api.getProgress(chapterId).then(setProgress).catch(() => {})
    api.getSettings().then(setSettings).catch(() => {})
  }, [chapterId])

  useEffect(() => {
    if (!chapter?.manga_id) return
    api.getManga(chapter.manga_id).then(setManga).catch(() => {})
  }, [chapter?.manga_id])

  // Effective mode: in-session override → manga setting → global setting → 'paged'
  const effectiveMode: 'paged' | 'scroll' =
    modeOverride ?? (manga?.reader_mode as 'paged' | 'scroll' | null) ?? settings?.reader_mode ?? 'paged'

  useEffect(() => {
    if (progress?.current_page != null && !progress.completed) {
      setPage(progress.current_page)
    }
  }, [progress])

  const knownTotal = chapter?.page_count ?? 0
  const total = knownTotal > 0 ? knownTotal : (lastPage != null ? lastPage : null)

  function goTo(p: number) {
    const clamped = total != null
      ? Math.max(0, Math.min(p, total - 1))
      : Math.max(0, p)
    setPage(clamped)
    const completed = total != null && clamped >= total - 1
    api.updateProgress(chapterId!, clamped, completed).catch(() => {})
  }

  useEffect(() => {
    if (effectiveMode !== 'paged') return
    function onKey(e: KeyboardEvent) {
      if (e.key === 'ArrowRight' || e.key === 'ArrowDown') goTo(page + 1)
      if (e.key === 'ArrowLeft'  || e.key === 'ArrowUp')   goTo(page - 1)
    }
    window.addEventListener('keydown', onKey)
    return () => window.removeEventListener('keydown', onKey)
  })

  if (!chapter) {
    return <div className="flex items-center justify-center h-full text-muted-foreground">Loading…</div>
  }

  const totalLabel = total != null ? String(total) : '?'
  const atEnd = total != null && page >= total - 1

  function toggleMode() {
    setModeOverride(m => (m ?? effectiveMode) === 'paged' ? 'scroll' : 'paged')
  }

  const ModeIcon = effectiveMode === 'scroll' ? BookOpen : AlignJustify

  return (
    <div className="reader-page">
      <header className="flex items-center gap-2 px-4 py-3 border-b border-border bg-card/90 backdrop-blur shrink-0">
        <Button variant="ghost" size="icon" onClick={() => navigate(ROUTES.manga(chapter.manga_id))} title="Back">
          <ChevronRight className="w-4 h-4 rotate-180" />
        </Button>
        <Button variant="ghost" size="icon" onClick={() => navigate(ROUTES.home)} title="Home">
          <Library className="w-4 h-4" />
        </Button>
        <p className="text-sm font-medium flex-1 truncate">
          Ch. {chapter.number}{chapter.title ? ` — ${chapter.title}` : ''}
        </p>
        <span className="text-xs text-muted-foreground shrink-0 mr-1">
          {effectiveMode === 'paged' ? `${page + 1} / ${totalLabel}` : totalLabel !== '?' ? `${totalLabel} pages` : ''}
        </span>
        <Button variant="ghost" size="icon" onClick={toggleMode} title={effectiveMode === 'paged' ? 'Switch to scroll' : 'Switch to paged'}>
          <ModeIcon className="w-4 h-4" />
        </Button>
      </header>

      {effectiveMode === 'paged' ? (
        <>
          <div className="reader-scroll" onClick={() => goTo(page + 1)}>
            <img
              key={page}
              src={api.pageUrl(chapterId!, page)}
              alt={`Page ${page + 1}`}
              onClick={(e) => { e.stopPropagation(); goTo(page + 1) }}
              onError={() => { if (page > 0) setLastPage(page) }}
            />
          </div>
          <footer className="flex items-center justify-center gap-4 px-4 py-3 border-t border-border bg-card/90 backdrop-blur shrink-0">
            <Button variant="outline" size="sm" disabled={page === 0} onClick={() => goTo(page - 1)} className="gap-1">
              <ChevronLeft className="w-3 h-3" /> Prev
            </Button>
            <span className="text-sm text-muted-foreground min-w-16 text-center">{page + 1} / {totalLabel}</span>
            <Button variant="outline" size="sm" disabled={atEnd} onClick={() => goTo(page + 1)} className="gap-1">
              Next <ChevronRight className="w-3 h-3" />
            </Button>
          </footer>
        </>
      ) : (
        <ScrollReader
          chapterId={chapterId!}
          total={total}
          onPageSeen={(p) => setPage(p)}
          onLastPageFailed={(p) => setLastPage(p)}
          initialPage={page}
        />
      )}
    </div>
  )
}

// ——— Scroll reader ———

function ScrollReader({
  chapterId,
  total,
  onPageSeen,
  onLastPageFailed,
  initialPage,
}: {
  chapterId: string
  total: number | null
  onPageSeen: (page: number) => void
  onLastPageFailed: (lastGood: number) => void
  initialPage: number
}) {
  // Render pages up to `rendered` — grow as user nears the end
  const knownMax = total != null ? total : 200
  const [rendered, setRendered] = useState(() => Math.min(knownMax, (total ?? 0) > 0 ? total! : 20))
  const [failed, setFailed] = useState<Set<number>>(new Set())
  const containerRef = useRef<HTMLDivElement>(null)
  const seenRef = useRef(-1)

  // Scroll to initial page on mount
  useEffect(() => {
    if (initialPage <= 0 || !containerRef.current) return
    // Approximate: each page ~500px; can't know exact until images load
    const approx = initialPage * 500
    containerRef.current.scrollTop = approx
  }, []) // only on mount

  function onScroll(e: React.UIEvent<HTMLDivElement>) {
    const el = e.currentTarget
    // Grow rendered pages as we near the bottom
    if (el.scrollTop + el.clientHeight > el.scrollHeight - 400) {
      setRendered(r => Math.min(knownMax, r + 5))
    }
    // Track approximate current page by scroll position
    const imgs = el.querySelectorAll<HTMLImageElement>('[data-page]')
    let current = 0
    for (const img of imgs) {
      const rect = img.getBoundingClientRect()
      const containerTop = el.getBoundingClientRect().top
      if (rect.top - containerTop < el.clientHeight * 0.5) {
        current = Number(img.dataset.page)
      }
    }
    if (current !== seenRef.current) {
      seenRef.current = current
      onPageSeen(current)
      const completed = total != null && current >= total - 1
      api.updateProgress(chapterId, current, completed).catch(() => {})
    }
  }

  const pages = Array.from({ length: rendered }, (_, i) => i)

  return (
    <div
      ref={containerRef}
      className="flex-1 overflow-y-auto bg-black"
      onScroll={onScroll}
    >
      <div className="flex flex-col items-center gap-1 py-2 max-w-3xl mx-auto">
        {pages.map((p) => failed.has(p) ? null : (
          <img
            key={p}
            data-page={p}
            src={api.pageUrl(chapterId, p)}
            alt={`Page ${p + 1}`}
            className={cn('w-full block', 'select-none')}
            onError={() => {
              if (p > 0) onLastPageFailed(p)
              setFailed(f => new Set(f).add(p))
            }}
          />
        ))}
      </div>
    </div>
  )
}
