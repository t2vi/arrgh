import { Search, ChevronRight, AlertCircle } from 'lucide-react'
import { Input } from '@/components/ui/input'
import { Button } from '@/components/ui/button'
import { ScrollArea } from '@/components/ui/scroll-area'
import { ROUTES } from '@/lib/routes'
import { useDiscover } from './hooks/useDiscover'
import { SearchRow } from './components/SearchRow'
import { ContentTypeFilter } from './components/ContentTypeFilter'
import { SearchSkeleton } from './components/SearchSkeleton'

export default function Discover() {
  const h = useDiscover()

  return (
    <div className="flex flex-col h-full">
      <header className="flex items-center gap-3 px-4 py-3 border-b border-border bg-card shrink-0">
        <Button variant="ghost" size="icon" onClick={() => h.navigate(-1)} title="Back">
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
                value={h.query}
                onChange={(e) => h.setQuery(e.target.value)}
                onKeyDown={(e) => e.key === 'Enter' && h.submit()}
                autoFocus
              />
            </div>
            <Button onClick={h.submit} disabled={h.isFetching}>
              {h.isFetching ? '…' : 'Search'}
            </Button>
          </div>

          <ContentTypeFilter
            value={h.contentType}
            onChange={h.setContentType}
            availableTypes={h.availableContentTypes}
          />

          {(h.searchError || h.addError) && (
            <div className="flex items-center gap-2 text-destructive text-sm rounded-lg border border-destructive/30 bg-destructive/10 px-3 py-2">
              <AlertCircle className="w-4 h-4 shrink-0" />
              {h.addError ?? h.searchError}
            </div>
          )}

          {h.isFetching && <SearchSkeleton />}

          {h.data && !h.isFetching && (
            <div className="space-y-3">
              {h.data.length === 0 && (
                <p className="text-muted-foreground text-sm">
                  {h.contentType && !h.availableContentTypes.has(h.contentType)
                    ? `No sources registered for "${h.contentType}". Add a plugin in Settings → Sources.`
                    : 'No results.'}
                </p>
              )}
              {h.data.map((r) => {
                const primaryKey = `${r.source}:${r.id}`
                const inLibraryNow = r.in_library || h.added.has(primaryKey) ||
                  r.alternatives.some((a) => h.added.has(`${a.source}:${a.id}`))
                const libraryId = r.library_id ??
                  h.added.get(primaryKey) ??
                  r.alternatives.map((a) => h.added.get(`${a.source}:${a.id}`)).find(Boolean)

                return (
                  <SearchRow
                    key={primaryKey}
                    result={r}
                    inLibrary={inLibraryNow}
                    addingKey={h.addingKey}
                    libraryId={libraryId}
                    onAdd={(alt) => h.handleAdd(r, alt)}
                    onView={(id) => h.navigate(ROUTES.manga(id))}
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
