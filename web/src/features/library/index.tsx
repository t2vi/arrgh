import { useNavigate } from 'react-router-dom'
import { Search, SlidersHorizontal, Plus } from 'lucide-react'
import { Input } from '@/components/ui/input'
import { Button } from '@/components/ui/button'
import { ROUTES } from '@/lib/routes'
import { useLibrary } from './hooks/useLibrary'
import { MangaCard } from './components/MangaCard'
import { MangaGridSkeleton } from './components/MangaGridSkeleton'
import { SortDropdown } from './components/SortDropdown'

const CONTENT_TYPE_OPTIONS = [
  { value: 'manga',   label: 'Manga' },
  { value: 'manhwa',  label: 'Manhwa' },
  { value: 'manhua',  label: 'Manhua' },
  { value: 'novel',   label: 'Novel' },
]

const STATUS_OPTIONS = [
  { value: 'ongoing',   label: 'Ongoing' },
  { value: 'completed', label: 'Completed' },
  { value: 'hiatus',    label: 'Hiatus' },
  { value: 'cancelled', label: 'Cancelled' },
]

export default function Library() {
  const navigate = useNavigate()
  const h = useLibrary()

  return (
    <>
      <header className="flex items-center gap-4 px-6 py-3 border-b border-border shrink-0 bg-background/80 backdrop-blur-xl sticky top-0 z-10">
        <div className="flex-1 max-w-sm">
          <div className="relative">
            <Search className="absolute left-3 top-1/2 -translate-y-1/2 w-4 h-4 text-muted-foreground pointer-events-none" />
            <Input
              className="pl-9 rounded-full bg-muted border-transparent focus-visible:border-ring"
              placeholder="Search your library…"
              value={h.search}
              onChange={(e) => { h.setSearch(e.target.value); h.setPage(() => 1) }}
            />
          </div>
        </div>

        <div className="flex-1" />

        <div className="flex items-center gap-2 shrink-0">
          <SortDropdown value={h.sort} onChange={h.setSort} />
          <Button
            variant="outline"
            size="sm"
            className={`gap-1.5 text-xs ${h.hasFilters ? 'border-primary text-primary' : ''}`}
            onClick={() => h.setShowFilters(!h.showFilters)}
          >
            <SlidersHorizontal className="w-3 h-3" />
            Filters
            {h.hasFilters && (
              <span className="bg-primary text-primary-foreground rounded-full w-4 h-4 text-[10px] flex items-center justify-center leading-none">
                {h.contentTypes.length + h.statuses.length}
              </span>
            )}
          </Button>
        </div>
      </header>

      {h.showFilters && (
        <div className="px-6 py-3 border-b border-border bg-muted/30 flex items-start gap-6 flex-wrap shrink-0">
          <div className="flex items-center gap-2 flex-wrap">
            <span className="text-xs text-muted-foreground shrink-0">Type</span>
            {CONTENT_TYPE_OPTIONS.map(({ value, label }) => (
              <button
                key={value}
                onClick={() => h.toggleContentType(value)}
                className={`px-2.5 py-0.5 rounded-full text-xs border transition-colors ${
                  h.contentTypes.includes(value)
                    ? 'bg-primary text-primary-foreground border-primary'
                    : 'border-border text-muted-foreground hover:border-foreground hover:text-foreground'
                }`}
              >
                {label}
              </button>
            ))}
          </div>
          <div className="flex items-center gap-2 flex-wrap">
            <span className="text-xs text-muted-foreground shrink-0">Status</span>
            {STATUS_OPTIONS.map(({ value, label }) => (
              <button
                key={value}
                onClick={() => h.toggleStatus(value)}
                className={`px-2.5 py-0.5 rounded-full text-xs border transition-colors ${
                  h.statuses.includes(value)
                    ? 'bg-primary text-primary-foreground border-primary'
                    : 'border-border text-muted-foreground hover:border-foreground hover:text-foreground'
                }`}
              >
                {label}
              </button>
            ))}
          </div>
          {h.hasFilters && (
            <button
              onClick={h.clearFilters}
              className="text-xs text-muted-foreground hover:text-foreground ml-auto self-center"
            >
              Clear all
            </button>
          )}
        </div>
      )}

      <div className="flex-1 overflow-auto">
        <div className="p-6 space-y-5">
          <div>
            <h2 className="text-2xl font-bold tracking-tight">My Library</h2>
            <p className="text-sm text-muted-foreground mt-0.5">
              {h.data ? `${h.data.total} saved ${h.data.total === 1 ? 'title' : 'titles'}` : ' '}
            </p>
          </div>

          {h.loading && <MangaGridSkeleton />}

          {h.data && (
            <>
              {h.data.items.length === 0 && (
                <p className="text-muted-foreground text-sm py-16 text-center">
                  {h.search || h.hasFilters ? 'No results.' : 'Library is empty — discover manga to add some.'}
                </p>
              )}

              <div className="grid grid-cols-[repeat(auto-fill,minmax(160px,1fr))] gap-5">
                {h.data.items.map((m) => (
                  <MangaCard
                    key={m.id}
                    manga={m}
                    onClick={() => navigate(ROUTES.title(m.id))}
                    onRemove={(deleteFiles) => h.handleRemove(m.id, deleteFiles)}
                    isRemoving={h.removingId === m.id}
                    syncMessage={h.syncMessages[m.id]}
                  />
                ))}
              </div>

              {h.totalPages > 1 && (
                <div className="flex items-center justify-center gap-3 pt-2">
                  <Button variant="outline" size="sm" disabled={h.page === 1} onClick={() => h.setPage((p) => p - 1)}>
                    Prev
                  </Button>
                  <span className="text-sm text-muted-foreground">{h.page} / {h.totalPages}</span>
                  <Button variant="outline" size="sm" disabled={h.page >= h.totalPages} onClick={() => h.setPage((p) => p + 1)}>
                    Next
                  </Button>
                </div>
              )}
            </>
          )}
        </div>
      </div>

      <button
        onClick={() => navigate(ROUTES.discover)}
        title="Discover manga"
        className="fixed bottom-6 right-6 w-12 h-12 rounded-full bg-primary text-primary-foreground flex items-center justify-center shadow-xl hover:opacity-90 transition-opacity z-10"
      >
        <Plus className="w-5 h-5" />
      </button>
    </>
  )
}
