import { useState, useEffect } from 'react'
import { useNavigate } from 'react-router-dom'
import { Search, ChevronRight, Plus, Check, AlertCircle, ChevronDown } from 'lucide-react'
import { api, type SearchResult, type MangaDetailResult, type SourceAlternative } from '@/api'
import { ROUTES } from '@/lib/routes'
import { Input } from '@/components/ui/input'
import { Button } from '@/components/ui/button'
import { Badge } from '@/components/ui/badge'
import { Skeleton } from '@/components/ui/skeleton'
import { ScrollArea } from '@/components/ui/scroll-area'
import { cn } from '@/lib/utils'

export default function Discover() {
  const [query, setQuery] = useState('')
  const [submitted, setSubmitted] = useState('')
  const [contentType, setContentType] = useState<string | undefined>()
  const navigate = useNavigate()

  // Track added items: "source:sourceId" → library manga id
  const [added, setAdded] = useState<Map<string, string>>(new Map())
  const [addError, setAddError] = useState<string | null>(null)
  const [addingKey, setAddingKey] = useState<string | null>(null)

  const [data, setData] = useState<SearchResult[] | undefined>()
  const [isFetching, setIsFetching] = useState(false)
  const [searchError, setSearchError] = useState(false)

  useEffect(() => {
    if (!submitted) return
    setIsFetching(true)
    setSearchError(false)
    api.searchManga(submitted, contentType)
      .then((r) => { setData(r); setSearchError(false) })
      .catch(() => { setSearchError(true) })
      .finally(() => setIsFetching(false))
  }, [submitted, contentType])

  async function handleAdd(result: SearchResult, alt?: { source: string; id: string; source_name: string }) {
    const source = alt?.source ?? result.source
    const sourceId = alt?.id ?? result.id
    const key = `${source}:${sourceId}`
    setAddError(null)
    setAddingKey(key)
    try {
      const manga = await api.addManga({ ...result, source, source_id: sourceId })
      setAdded((prev) => {
        const next = new Map(prev)
        // mark primary + selected alt as added
        next.set(`${result.source}:${result.id}`, manga.id)
        if (alt) next.set(key, manga.id)
        return next
      })
      navigate(ROUTES.manga(manga.id))
    } catch (err) {
      setAddError(err instanceof Error ? err.message : 'Failed to add manga')
    } finally {
      setAddingKey(null)
    }
  }

  function submit() {
    const q = query.trim()
    if (q) setSubmitted(q)
  }

  return (
    <div className="flex flex-col h-full">
      <header className="flex items-center gap-3 px-4 py-3 border-b border-border bg-card shrink-0">
        <Button variant="ghost" size="icon" onClick={() => navigate(-1)} title="Back">
          <ChevronRight className="w-4 h-4 rotate-180" />
        </Button>
        <h1 className="text-base font-semibold">Discover</h1>
      </header>

      <ScrollArea className="flex-1">
        <div className="p-4 space-y-4">
          <div className="flex gap-2">
            <div className="relative flex-1">
              <Search className="absolute left-3 top-1/2 -translate-y-1/2 w-4 h-4 text-muted-foreground" />
              <Input
                className="pl-9"
                placeholder="Search all sources…"
                value={query}
                onChange={(e) => setQuery(e.target.value)}
                onKeyDown={(e) => e.key === 'Enter' && submit()}
                autoFocus
              />
            </div>
            <Button onClick={submit} disabled={isFetching}>
              {isFetching ? '…' : 'Search'}
            </Button>
          </div>

          <ContentTypeFilter value={contentType} onChange={setContentType} />

          {(searchError || addError) && (
            <div className="flex items-center gap-2 text-destructive text-sm rounded-lg border border-destructive/30 bg-destructive/10 px-3 py-2">
              <AlertCircle className="w-4 h-4 shrink-0" />
              {addError ?? 'Search failed. Is the server running?'}
            </div>
          )}

          {isFetching && <SearchSkeleton />}

          {data && !isFetching && (
            <div className="space-y-3">
              {data.length === 0 && (
                <p className="text-muted-foreground text-sm">No results.</p>
              )}
              {data.map((r) => {
                const primaryKey = `${r.source}:${r.id}`
                const inLibraryNow = r.in_library || added.has(primaryKey) ||
                  r.alternatives.some((a) => added.has(`${a.source}:${a.id}`))
                const libraryId = r.library_id ??
                  added.get(primaryKey) ??
                  r.alternatives.map((a) => added.get(`${a.source}:${a.id}`)).find(Boolean)

                return (
                  <SearchRow
                    key={primaryKey}
                    result={r}
                    inLibrary={inLibraryNow}
                    addingKey={addingKey}
                    libraryId={libraryId}
                    onAdd={(alt) => handleAdd(r, alt)}
                    onView={(id) => navigate(ROUTES.manga(id))}
                  />
                )
              })}
            </div>
          )}
        </div>
      </ScrollArea>
    </div>
  )
}

function SourceBadge({ name }: { name: string }) {
  return (
    <span className="inline-flex items-center px-1.5 py-px rounded text-[10px] font-semibold bg-primary/15 text-primary">
      {name}
    </span>
  )
}

function SearchRow({
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
    api.getDiscoverDetail(result.source, result.id)
      .then(setDetail)
      .catch(() => {})
  }, [needsDetail, result.source, result.id])

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
          {/* Source badge + picker */}
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
                {/* Primary option */}
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

const CONTENT_TYPES = [
  { value: undefined,   label: 'All'     },
  { value: 'manga',     label: 'Manga'   },
  { value: 'manhwa',   label: 'Manhwa'  },
  { value: 'manhua',   label: 'Manhua'  },
  { value: 'one-shot', label: 'One-shot' },
] as const

function ContentTypeFilter({
  value,
  onChange,
}: {
  value: string | undefined
  onChange: (v: string | undefined) => void
}) {
  return (
    <div className="flex gap-1 flex-wrap">
      {CONTENT_TYPES.map((ct) => (
        <button
          key={ct.label}
          type="button"
          onClick={() => onChange(ct.value)}
          className={cn(
            'px-2.5 py-0.5 rounded-full text-xs font-medium transition-colors border',
            value === ct.value
              ? 'bg-primary text-primary-foreground border-primary'
              : 'bg-muted text-muted-foreground border-transparent hover:border-border hover:text-foreground',
          )}
        >
          {ct.label}
        </button>
      ))}
    </div>
  )
}

function SearchSkeleton() {
  return (
    <div className="space-y-3">
      {Array.from({ length: 5 }).map((_, i) => (
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
  )
}
