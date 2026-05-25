import { Check, Plus } from 'lucide-react'
import { api, type SearchResult } from '@/api'
import { Button } from '@/components/ui/button'
import { Badge } from '@/components/ui/badge'
import { ContentTypePill } from '@/components/ContentTypePill'

export function SearchRow({
  result, inLibrary, addingId, libraryId, onAdd, onView,
}: {
  result: SearchResult
  inLibrary: boolean
  addingId: string | null
  libraryId?: string
  onAdd: () => void
  onView: (id: string) => void
}) {
  const coverSrc = result.cover_url ? api.proxyImageUrl(result.cover_url) : null
  const isAdding = addingId === result.mangaupdates_id

  const isExplicit = result.tags?.split(',').some((t) => t.trim().toLowerCase() === 'adult') ?? false

  return (
    <div className="flex gap-3 rounded-lg border border-border bg-card p-3">
      {coverSrc ? (
        <img
          src={coverSrc}
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
          <ContentTypePill type={result.content_type} />
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
          {result.year && (
            <span className="text-xs text-muted-foreground">{result.year}</span>
          )}
        </div>

        {result.description ? (
          <p className="text-xs text-muted-foreground line-clamp-3">{result.description}</p>
        ) : (
          <div className="space-y-1 pt-0.5">
            <div className="h-2.5 w-full bg-muted rounded animate-pulse" />
            <div className="h-2.5 w-4/5 bg-muted rounded animate-pulse" />
          </div>
        )}
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
