import { useNavigate } from 'react-router-dom'
import { Search, ChevronDown, SlidersHorizontal, Plus } from 'lucide-react'
import { Input } from '@/components/ui/input'
import { Button } from '@/components/ui/button'
import { ROUTES } from '@/lib/routes'
import { useLibrary } from './hooks/useLibrary'
import { MangaCard } from './components/MangaCard'
import { MangaGridSkeleton } from './components/MangaGridSkeleton'

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
          <Button variant="outline" size="sm" className="gap-1.5 text-xs">
            <span className="text-muted-foreground">Sort by:</span>
            Recently Added
            <ChevronDown className="w-3 h-3 text-muted-foreground" />
          </Button>
          <Button variant="outline" size="sm" className="gap-1.5 text-xs">
            <SlidersHorizontal className="w-3 h-3" />
            Filters
          </Button>
        </div>
      </header>

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
                  {h.search ? 'No results.' : 'Library is empty — discover manga to add some.'}
                </p>
              )}

              <div className="grid grid-cols-[repeat(auto-fill,minmax(160px,1fr))] gap-5">
                {h.data.items.map((m) => (
                  <MangaCard
                    key={m.id}
                    manga={m}
                    onClick={() => navigate(ROUTES.manga(m.id))}
                    onRemove={(deleteFiles) => h.handleRemove(m.id, deleteFiles)}
                    isRemoving={h.removingId === m.id}
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
