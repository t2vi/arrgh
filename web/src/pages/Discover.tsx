import { useState } from 'react'
import { useNavigate } from 'react-router-dom'
import { useQuery, useMutation, useQueryClient } from '@tanstack/react-query'
import { Search, ChevronRight, Plus, Check, AlertCircle } from 'lucide-react'
import { api, type SearchResult, type MangaDetailResult } from '@/api'
import { ROUTES } from '@/lib/routes'
import { Input } from '@/components/ui/input'
import { Button } from '@/components/ui/button'
import { Badge } from '@/components/ui/badge'
import { Skeleton } from '@/components/ui/skeleton'
import { ScrollArea } from '@/components/ui/scroll-area'

export default function Discover() {
  const [query, setQuery] = useState('')
  const [submitted, setSubmitted] = useState('')
  const navigate = useNavigate()
  const qc = useQueryClient()

  const [added, setAdded] = useState<Map<string, string>>(new Map())
  const [addError, setAddError] = useState<string | null>(null)

  const { data, isFetching, error } = useQuery({
    queryKey: ['discover', submitted],
    queryFn: () => api.searchManga(submitted),
    enabled: submitted.length > 0,
  })

  const add = useMutation({
    mutationFn: api.addManga,
    onSuccess: (manga, result) => {
      setAdded((prev) => new Map(prev).set(result.id, manga.id))
      setAddError(null)
      qc.invalidateQueries({ queryKey: ['manga'] })
      navigate(ROUTES.manga(manga.id))
    },
    onError: (err) => {
      setAddError(err instanceof Error ? err.message : 'Failed to add manga')
    },
  })

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
                placeholder="Search Mangapill…"
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

          {(error || addError) && (
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
                const libraryId = added.get(r.id) ?? r.library_id
                const inLibrary = r.in_library || added.has(r.id)
                const isAdding = add.isPending && add.variables?.id === r.id
                return (
                  <SearchRow
                    key={r.id}
                    result={r}
                    inLibrary={inLibrary}
                    isAdding={isAdding}
                    libraryId={libraryId}
                    onAdd={() => { setAddError(null); add.mutate(r) }}
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

function SearchRow({
  result, inLibrary, isAdding, libraryId, onAdd, onView,
}: {
  result: SearchResult
  inLibrary: boolean
  isAdding: boolean
  libraryId?: string
  onAdd: () => void
  onView: (id: string) => void
}) {
  const coverSrc = result.cover_url ? api.proxyImageUrl(result.cover_url) : null

  // Mangapill search doesn't include description — lazy-fetch detail
  const needsDetail = !result.description
  const { data: detail } = useQuery<MangaDetailResult>({
    queryKey: ['discover-detail', result.id],
    queryFn: () => api.getDiscoverDetail(result.source, result.id),
    enabled: needsDetail,
    staleTime: 5 * 60 * 1000,
  })

  const description = result.description ?? detail?.description
  const resolvedCover = coverSrc
    ?? (detail?.cover_url ? api.proxyImageUrl(detail.cover_url) : null)

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

      <div className="flex-1 min-w-0 space-y-1">
        <p className="font-medium text-sm leading-tight line-clamp-2">{result.title}</p>
        <div className="flex gap-1.5 flex-wrap items-center">
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
          <Button size="sm" onClick={onAdd} disabled={isAdding} className="gap-1">
            {isAdding ? '…' : <><Plus className="w-3 h-3" /> Add</>}
          </Button>
        )}
      </div>
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
