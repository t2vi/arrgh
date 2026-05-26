import { useEffect, useState } from 'react'
import { ChevronRight, ChevronLeft, Library, Loader2 } from 'lucide-react'
import { api } from '@/api'
import { ROUTES } from '@/lib/routes'
import { Button } from '@/components/ui/button'
import { useReader } from './hooks/useReader'
import { ScrollReader } from './components/ScrollReader'
import { NovelReader } from './components/NovelReader'

export default function Reader() {
  const h = useReader()
  const [imgLoading, setImgLoading] = useState(true)

  useEffect(() => { setImgLoading(true) }, [h.page])

  if (!h.chapter) {
    return <div className="flex items-center justify-center h-full text-muted-foreground">Loading…</div>
  }

  return (
    <div className="reader-page">
      <header className="flex items-center gap-2 px-4 py-3 border-b border-border bg-card/90 backdrop-blur shrink-0">
        <Button variant="ghost" size="icon" onClick={() => h.navigate(ROUTES.title(h.chapter!.title_id))} title="Back">
          <ChevronRight className="w-4 h-4 rotate-180" />
        </Button>
        <Button variant="ghost" size="icon" onClick={() => h.navigate(ROUTES.home)} title="Home">
          <Library className="w-4 h-4" />
        </Button>
        <p className="text-sm font-medium flex-1 truncate">
          Ch. {h.chapter.number}{h.chapter.title ? ` — ${h.chapter.title}` : ''}
        </p>
        {!h.isNovel && (
          <>
            <span className="text-xs text-muted-foreground shrink-0 mr-1">
              {h.effectiveMode === 'paged'
                ? `${h.page + 1} / ${h.totalLabel}`
                : h.totalLabel !== '?' ? `${h.totalLabel} pages` : ''}
            </span>
            <Button
              variant="ghost"
              size="icon"
              onClick={h.toggleMode}
              title={h.effectiveMode === 'paged' ? 'Switch to scroll' : 'Switch to paged'}
            >
              <h.ModeIcon className="w-4 h-4" />
            </Button>
          </>
        )}
      </header>

      {h.isNovel ? (
        h.novelContent != null
          ? <NovelReader content={h.novelContent} onRead={h.markNovelRead} />
          : h.novelError
          ? (
            <div className="flex flex-col items-center justify-center h-full gap-3 text-center px-6">
              <p className="text-muted-foreground text-sm">Chapter not downloaded.</p>
              <p className="text-muted-foreground/60 text-xs">Go back to the manga page and download this chapter first.</p>
              <button
                onClick={() => h.navigate(-1)}
                className="mt-2 text-sm text-primary hover:underline"
              >
                Go back
              </button>
            </div>
          )
          : <div className="flex items-center justify-center h-full text-muted-foreground">Loading…</div>
      ) : h.effectiveMode === 'paged' ? (
        <>
          <div className="reader-scroll" onClick={() => h.goTo(h.page + 1)}>
            {imgLoading && (
              <div className="absolute inset-0 flex items-center justify-center pointer-events-none">
                <Loader2 className="w-8 h-8 animate-spin text-muted-foreground/60" />
              </div>
            )}
            <img
              key={h.page}
              src={api.pageUrl(h.chapterId!, h.page)}
              alt={`Page ${h.page + 1}`}
              onClick={(e) => { e.stopPropagation(); h.goTo(h.page + 1) }}
              onLoad={() => setImgLoading(false)}
              onError={() => { setImgLoading(false); if (h.page > 0) h.setLastPage(h.page) }}
            />
          </div>
          <footer className="flex items-center justify-center gap-4 px-4 py-3 border-t border-border bg-card/90 backdrop-blur shrink-0">
            <Button variant="outline" size="sm" disabled={h.page === 0} onClick={() => h.goTo(h.page - 1)} className="gap-1">
              <ChevronLeft className="w-3 h-3" /> Prev
            </Button>
            <span className="text-sm text-muted-foreground min-w-16 text-center">
              {h.page + 1} / {h.totalLabel}
            </span>
            <Button variant="outline" size="sm" disabled={h.atEnd} onClick={() => h.goTo(h.page + 1)} className="gap-1">
              Next <ChevronRight className="w-3 h-3" />
            </Button>
          </footer>
        </>
      ) : (
        <ScrollReader
          chapterId={h.chapterId!}
          total={h.total}
          onPageSeen={(p) => h.goTo(p)}
          onLastPageFailed={(p) => h.setLastPage(p)}
          initialPage={h.page}
        />
      )}
    </div>
  )
}
