import { useEffect, useRef, useState } from 'react'
import { Loader2 } from 'lucide-react'
import { api } from '@/api'
import { cn } from '@/lib/utils'

export function ScrollReader({
  chapterId,
  total,
  onPageSeen,
  onLastPageFailed,
  initialPage,
  zoom = 100,
}: {
  chapterId: string
  total: number | null
  onPageSeen: (page: number) => void
  onLastPageFailed: (lastGood: number) => void
  initialPage: number
  zoom?: number
}) {
  const knownMax = total != null ? total : 200
  const [rendered, setRendered] = useState(() => Math.min(knownMax, (total ?? 0) > 0 ? total! : 20))
  const [failed, setFailed] = useState<Set<number>>(new Set())
  const [loaded, setLoaded] = useState<Set<number>>(new Set())
  const containerRef = useRef<HTMLDivElement>(null)
  const seenRef = useRef(-1)

  useEffect(() => {
    if (initialPage <= 0 || !containerRef.current) return
    const approx = initialPage * 500
    containerRef.current.scrollTop = approx
  }, []) // only on mount

  function onScroll(e: React.UIEvent<HTMLDivElement>) {
    const el = e.currentTarget
    if (el.scrollTop + el.clientHeight > el.scrollHeight - 400) {
      setRendered((r) => Math.min(knownMax, r + 5))
    }
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
      <div className="flex flex-col items-center gap-1 py-2 w-full mx-auto" style={{ maxWidth: `${zoom * 8}px` }}>
        {pages.map((p) => failed.has(p) ? null : (
          <div key={p} className="relative w-full">
            {!loaded.has(p) && (
              <div className="w-full flex items-center justify-center bg-white/5" style={{ minHeight: 480 }}>
                <Loader2 className="w-7 h-7 animate-spin text-muted-foreground/50" />
              </div>
            )}
            <img
              data-page={p}
              src={api.pageUrl(chapterId, p)}
              alt={`Page ${p + 1}`}
              className={cn('w-full block select-none', !loaded.has(p) && 'h-0 overflow-hidden')}
              onLoad={() => setLoaded((l) => new Set(l).add(p))}
              onError={() => {
                setLoaded((l) => new Set(l).add(p))
                if (p > 0) onLastPageFailed(p)
                setFailed((f) => new Set(f).add(p))
              }}
            />
          </div>
        ))}
      </div>
    </div>
  )
}
