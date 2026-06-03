import { useState, useEffect } from 'react'
import { Skeleton } from '@/components/ui/skeleton'

// Sources queried in parallel during a discover fan-out.
// Order matches AuthorityOrder in server/Api/Discover.cs.
const SOURCES = [
  { key: 'mangaupdates', label: 'MangaUpdates' },
  { key: 'anilist',       label: 'AniList' },
  { key: 'mangadex',      label: 'MangaDex' },
  { key: 'novelupdates',  label: 'NovelUpdates' },
  { key: 'wuxiaworld',    label: 'WuxiaWorld' },
  { key: 'nhentai',       label: 'nhentai' },
]

export function SearchProgress() {
  // Stagger the sources appearing so the user sees activity, not just a static list.
  const [visible, setVisible] = useState(0)

  useEffect(() => {
    if (visible >= SOURCES.length) return
    const t = setTimeout(() => setVisible((v) => v + 1), 120)
    return () => clearTimeout(t)
  }, [visible])

  return (
    <div className="space-y-4">
      <div className="rounded-lg border border-border bg-card px-4 py-3">
        <p className="text-xs font-medium text-muted-foreground uppercase tracking-wider mb-3">
          Searching sources…
        </p>
        <div className="flex flex-wrap gap-2">
          {SOURCES.map((s, i) => (
            <span
              key={s.key}
              className={[
                'flex items-center gap-1.5 rounded-full border px-2.5 py-1 text-xs font-medium transition-all duration-300',
                i < visible
                  ? 'border-primary/40 bg-primary/10 text-primary opacity-100 translate-y-0'
                  : 'border-border bg-muted text-muted-foreground opacity-0 translate-y-1',
              ].join(' ')}
            >
              <span
                className={[
                  'w-1.5 h-1.5 rounded-full',
                  i < visible ? 'bg-primary animate-pulse' : 'bg-muted-foreground/40',
                ].join(' ')}
              />
              {s.label}
            </span>
          ))}
        </div>
      </div>

      {/* skeleton rows below the source progress */}
      <div className="space-y-3">
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
    </div>
  )
}
