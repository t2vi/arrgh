import { useState, useEffect } from 'react'
import { Skeleton } from '@/components/ui/skeleton'
import { cn } from '@/lib/utils'

const SOURCES = [
  { key: 'mangaupdates', label: 'MangaUpdates' },
  { key: 'anilist',       label: 'AniList' },
  { key: 'mangadex',      label: 'MangaDex' },
  { key: 'novelupdates',  label: 'NovelUpdates' },
  { key: 'wuxiaworld',    label: 'WuxiaWorld' },
  { key: 'nhentai',       label: 'nhentai' },
]

interface Props {
  /** When defined, search is done — pills flip to green/grey based on which sources contributed. */
  completedSources?: Set<string>
}

export function SearchProgress({ completedSources }: Props) {
  const isDone = completedSources !== undefined
  const [visible, setVisible] = useState(0)
  const [settled, setSettled] = useState(0)

  // Stagger purple pills appearing while searching
  useEffect(() => {
    if (isDone || visible >= SOURCES.length) return
    const t = setTimeout(() => setVisible((v) => v + 1), 120)
    return () => clearTimeout(t)
  }, [visible, isDone])

  // Stagger green/grey transition when results arrive
  useEffect(() => {
    if (!isDone) { setSettled(0); return }
    if (settled >= SOURCES.length) return
    const t = setTimeout(() => setSettled((v) => v + 1), 100)
    return () => clearTimeout(t)
  }, [isDone, settled])

  return (
    <div className="space-y-4">
      <div className="rounded-lg border border-border bg-card px-4 py-3">
        <p className="text-xs font-medium text-muted-foreground uppercase tracking-wider mb-3">
          {isDone ? 'Results from…' : 'Searching sources…'}
        </p>
        <div className="flex flex-wrap gap-2">
          {SOURCES.map((s, i) => {
            const isVisible = isDone || i < visible
            const hasSettled = isDone && i < settled
            const hasResult = hasSettled && completedSources.has(s.key)

            return (
              <span
                key={s.key}
                data-testid={`source-pill-${s.key}`}
                className={cn(
                  'flex items-center gap-1.5 rounded-full border px-2.5 py-1 text-xs font-medium transition-all duration-300',
                  !isVisible && 'opacity-0 translate-y-1',
                  isVisible && !hasSettled && 'border-primary/40 bg-primary/10 text-primary',
                  hasSettled && hasResult && 'border-green-500/40 bg-green-500/10 text-green-400',
                  hasSettled && !hasResult && 'border-border bg-muted text-muted-foreground opacity-50',
                )}
              >
                <span
                  data-testid="source-dot"
                  className={cn(
                    'w-1.5 h-1.5 rounded-full',
                    isVisible && !hasSettled && 'bg-primary animate-pulse',
                    hasSettled && hasResult && 'bg-green-400',
                    hasSettled && !hasResult && 'bg-muted-foreground/40',
                  )}
                />
                {s.label}
              </span>
            )
          })}
        </div>
      </div>

      {!isDone && (
        <div data-testid="skeleton-rows" className="space-y-3">
          {Array.from({ length: 4 }).map((_, i) => (
            <div key={i} className="flex gap-3 rounded-lg border border-border bg-card p-3">
              <Skeleton className="w-14 shrink-0 rounded aspect-[2/3]" />
              <div className="flex-1 space-y-2">
                <Skeleton className="h-4 w-3/4" />
                <Skeleton className="h-3 w-1/4" />
                <Skeleton className="h-3 w-full" />
                <Skeleton className="h-3 w-5/6" />
              </div>
            </div>
          ))}
        </div>
      )}
    </div>
  )
}
