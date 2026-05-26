import { ChevronLeft, ChevronRight } from 'lucide-react'
import { Button } from '@/components/ui/button'
import { ROUTES } from '@/lib/routes'
import type { Chapter } from '@/types'
import type { NavigateFunction } from 'react-router-dom'

interface Props {
  mode: 'paged' | 'scroll' | 'novel'
  page: number
  total: number | null
  totalLabel: string
  atEnd: boolean
  prevChapter: Chapter | null
  nextChapter: Chapter | null
  navigate: NavigateFunction
  onPrevPage: () => void
  onNextPage: () => void
}

export function ReaderFooter({
  mode, page, totalLabel, atEnd,
  prevChapter, nextChapter,
  navigate, onPrevPage, onNextPage,
}: Props) {
  const atStart = page === 0

  if (mode === 'paged') {
    const prevAction = atStart && prevChapter
      ? () => navigate(ROUTES.reader(prevChapter.id))
      : atStart ? null : onPrevPage

    const nextAction = atEnd && nextChapter
      ? () => navigate(ROUTES.reader(nextChapter.id))
      : atEnd ? null : onNextPage

    const prevLabel = atStart && prevChapter ? 'Prev Ch.' : 'Prev'
    const nextLabel = atEnd && nextChapter ? 'Next Ch.' : 'Next'

    return (
      <footer className="flex items-center justify-center gap-4 px-4 py-3 border-t border-border bg-card/90 backdrop-blur shrink-0">
        <Button variant="outline" size="sm" disabled={!prevAction} onClick={prevAction ?? undefined} className="gap-1">
          <ChevronLeft className="w-3 h-3" /> {prevLabel}
        </Button>
        <span className="text-sm text-muted-foreground min-w-16 text-center">
          {page + 1} / {totalLabel}
        </span>
        <Button variant="outline" size="sm" disabled={!nextAction} onClick={nextAction ?? undefined} className="gap-1">
          {nextLabel} <ChevronRight className="w-3 h-3" />
        </Button>
      </footer>
    )
  }

  return (
    <footer className="flex items-center justify-center gap-4 px-4 py-3 border-t border-border bg-card/90 backdrop-blur shrink-0">
      <Button
        variant="outline" size="sm"
        disabled={!prevChapter}
        onClick={() => prevChapter && navigate(ROUTES.reader(prevChapter.id))}
        className="gap-1"
      >
        <ChevronLeft className="w-3 h-3" /> Prev Ch.
      </Button>
      <Button
        variant="outline" size="sm"
        disabled={!nextChapter}
        onClick={() => nextChapter && navigate(ROUTES.reader(nextChapter.id))}
        className="gap-1"
      >
        Next Ch. <ChevronRight className="w-3 h-3" />
      </Button>
    </footer>
  )
}
