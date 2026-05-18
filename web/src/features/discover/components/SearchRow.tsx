import { useState, useEffect } from 'react'
import { Check, Plus, ChevronDown } from 'lucide-react'
import { api, type SearchResult, type MangaDetailResult, type SourceAlternative } from '@/api'
import { Button } from '@/components/ui/button'
import { Badge } from '@/components/ui/badge'
import { cn } from '@/lib/utils'

function SourceBadge({ name }: { name: string }) {
  return (
    <span className="inline-flex items-center px-1.5 py-px rounded text-[10px] font-semibold bg-primary/15 text-primary">
      {name}
    </span>
  )
}

export function SearchRow({
  result, inLibrary, addingKey, libraryId, onAdd, onView,
}: {
  result: SearchResult
  inLibrary: boolean
  addingKey: string | null
  libraryId?: string
  onAdd: (alt?: { source: string; id: string; source_name: string }) => void
  onView: (id: string) => void
}) {
  const coverSrc = result.cover_url ? api.proxyImageUrl(result.cover_url) : null
  const [pickerOpen, setPickerOpen] = useState(false)
  const [selectedAlt, setSelectedAlt] = useState<SourceAlternative | null>(null)

  const needsDetail = !result.description
  const [detail, setDetail] = useState<MangaDetailResult | undefined>()

  useEffect(() => {
    if (!needsDetail) return
    api.getDiscoverDetail(result.source, result.id, result.title)
      .then(setDetail)
      .catch(() => {})
  }, [needsDetail, result.source, result.id])

  const explicitTags = result.tags ?? detail?.tags ?? null
  const isExplicit = explicitTags?.split(',').some((t) => t.trim().toLowerCase() === 'adult') ?? false
  const description = result.description ?? detail?.description
  const resolvedCover = coverSrc
    ?? (detail?.cover_url ? api.proxyImageUrl(detail.cover_url) : null)

  const activeSrc = selectedAlt
    ? { source: selectedAlt.source, id: selectedAlt.id, source_name: selectedAlt.source_name }
    : undefined

  const activeKey = activeSrc
    ? `${activeSrc.source}:${activeSrc.id}`
    : `${result.source}:${result.id}`
  const isAdding = addingKey === activeKey

  return (
    <div className="flex gap-3 rounded-lg border border-border bg-card p-3">
      {resolvedCover ? (
        <img
          src={resolvedCover}
          alt=""
          className="w-14 shrink-0 rounded aspect-[2/3] object-cover bg-muted"
          onError={(e) => { (e.target as HTMLImageElement).style.visibility = 'hidden' }}
        />
      ) : (
        <div className="w-14 shrink-0 rounded aspect-[2/3] bg-muted animate-pulse" />
      )}

      <div className="flex-1 min-w-0 space-y-1.5">
        <p className="font-medium text-sm leading-tight line-clamp-2">{result.title}</p>

        <div className="flex gap-1.5 flex-wrap items-center">
          <div className="relative">
            <button
              type="button"
              onClick={() => result.alternatives.length > 0 && setPickerOpen((v) => !v)}
              className={cn(
                'flex items-center gap-0.5',
                result.alternatives.length > 0 && 'cursor-pointer hover:opacity-80',
              )}
              title={result.alternatives.length > 0 ? 'Pick source' : undefined}
            >
              <SourceBadge name={selectedAlt ? selectedAlt.source_name : result.source_name} />
              {result.alternatives.length > 0 && (
                <span className="text-[10px] text-muted-foreground font-medium">
                  +{result.alternatives.length}
                  <ChevronDown className="inline w-2.5 h-2.5 ml-0.5" />
                </span>
              )}
            </button>

            {pickerOpen && (
              <div className="absolute left-0 top-full mt-1 z-20 bg-card border border-border rounded-lg shadow-lg py-1 min-w-[140px]">
                <button
                  type="button"
                  className={cn(
                    'w-full text-left px-3 py-1.5 text-xs flex items-center gap-2 hover:bg-muted/50 transition-colors',
                    !selectedAlt && 'text-primary font-medium',
                  )}
                  onClick={() => { setSelectedAlt(null); setPickerOpen(false) }}
                >
                  {!selectedAlt && <Check className="w-3 h-3 shrink-0" />}
                  <span className={!selectedAlt ? '' : 'ml-5'}>{result.source_name}</span>
                </button>
                {result.alternatives.map((alt) => (
                  <button
                    key={`${alt.source}:${alt.id}`}
                    type="button"
                    className={cn(
                      'w-full text-left px-3 py-1.5 text-xs flex items-center gap-2 hover:bg-muted/50 transition-colors',
                      selectedAlt?.source === alt.source && 'text-primary font-medium',
                    )}
                    onClick={() => { setSelectedAlt(alt); setPickerOpen(false) }}
                  >
                    {selectedAlt?.source === alt.source
                      ? <Check className="w-3 h-3 shrink-0" />
                      : <span className="w-3 shrink-0" />
                    }
                    {alt.source_name}
                  </button>
                ))}
              </div>
            )}
          </div>

          {isExplicit && (
            <span className="inline-flex items-center px-1.5 py-px rounded text-[10px] font-bold bg-red-500/15 text-red-400">
              18+
            </span>
          )}
          {result.status && result.status !== 'unknown' && (
            <Badge variant="secondary" className="capitalize text-xs">{result.status}</Badge>
          )}
          {result.author && (
            <span className="text-xs text-muted-foreground">{result.author}</span>
          )}
          {detail?.chapter_count ? (
            <span className="text-xs text-muted-foreground">{detail.chapter_count} ch.</span>
          ) : null}
        </div>

        {description ? (
          <p className="text-xs text-muted-foreground line-clamp-3">{description}</p>
        ) : needsDetail && !detail ? (
          <div className="space-y-1 pt-0.5">
            <div className="h-2.5 w-full bg-muted rounded animate-pulse" />
            <div className="h-2.5 w-4/5 bg-muted rounded animate-pulse" />
          </div>
        ) : null}
      </div>

      <div className="shrink-0 flex items-start pt-0.5">
        {inLibrary ? (
          <Button size="sm" variant="secondary" onClick={() => libraryId && onView(libraryId)} className="gap-1">
            <Check className="w-3 h-3" />
            In Library
          </Button>
        ) : (
          <Button size="sm" onClick={() => { setPickerOpen(false); onAdd(activeSrc) }} disabled={isAdding} className="gap-1">
            {isAdding ? '…' : <><Plus className="w-3 h-3" /> Add</>}
          </Button>
        )}
      </div>
    </div>
  )
}
