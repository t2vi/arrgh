import { useNavigate } from 'react-router-dom'
import { Plus } from 'lucide-react'
import { Button } from '@/components/ui/button'
import { ROUTES } from '@/lib/routes'
import { useHome } from './hooks/useHome'
import { GreetingJumbotron } from './components/GreetingJumbotron'
import {
  HomeSkeleton, ContinueCard, LibraryCoverCard,
  NewReleaseCard, RecentCard, TrendingCard, TrendingModal,
} from './components/Cards'

export default function Home() {
  const navigate = useNavigate()
  const h = useHome()

  return (
    <>
      <div className="flex-1 overflow-auto">
        {h.isLoading ? (
          <HomeSkeleton />
        ) : h.items.length === 0 ? (
          <div className="flex flex-col items-center justify-center h-full py-24 gap-4">
            <p className="text-muted-foreground text-sm">Library empty — discover titles to add some.</p>
            <Button onClick={() => navigate(ROUTES.discover)}>Discover</Button>
          </div>
        ) : (
          <div className="pb-12">
            <GreetingJumbotron
              typeCounts={h.items.reduce<Record<string, number>>((acc, m) => {
                acc[m.content_type] = (acc[m.content_type] ?? 0) + 1
                return acc
              }, {})}
              totalRead={h.totalRead}
              coverManga={h.coverManga}
            />

            {h.continueItems.length > 0 && (
              <section className="mt-8 px-6 space-y-4">
                <h2 className="text-xl font-bold">Continue Reading</h2>
                <div className="flex gap-4 overflow-x-auto pb-2 -mx-6 px-6 [&::-webkit-scrollbar]:hidden [-ms-overflow-style:none] [scrollbar-width:none]">
                  {h.continueItems.map((item) => (
                    <ContinueCard
                      key={item.chapter_id}
                      item={item}
                      onPlay={() => navigate(ROUTES.reader(item.chapter_id))}
                      onDetail={() => navigate(ROUTES.manga(item.manga_id))}
                    />
                  ))}
                </div>
              </section>
            )}

            <section className="mt-8 px-6 space-y-4">
              <div className="flex items-center justify-between">
                <h2 className="text-xl font-bold">My Library</h2>
                <button
                  onClick={() => navigate(ROUTES.library)}
                  className="text-sm text-primary hover:opacity-80 transition-opacity font-medium"
                >
                  View All
                </button>
              </div>
              <div className="flex gap-4 overflow-x-auto pb-2 -mx-6 px-6 [&::-webkit-scrollbar]:hidden [-ms-overflow-style:none] [scrollbar-width:none]">
                {h.items.slice(0, 10).map((m) => (
                  <LibraryCoverCard key={m.id} manga={m} onClick={() => navigate(ROUTES.manga(m.id))} />
                ))}
              </div>
            </section>

            {h.newReleases.length > 0 && (
              <section className="mt-8 px-6 space-y-4">
                <div className="flex items-center justify-between">
                  <h2 className="text-xl font-bold">New Releases</h2>
                  <span className="text-xs text-muted-foreground">{h.newReleases.length} new</span>
                </div>
                <div className="flex gap-3 overflow-x-auto pb-2 -mx-6 px-6 [&::-webkit-scrollbar]:hidden [-ms-overflow-style:none] [scrollbar-width:none]">
                  {h.newReleases.map((r) => (
                    <NewReleaseCard
                      key={r.chapter_id}
                      item={r}
                      onClick={() => navigate(ROUTES.reader(r.chapter_id))}
                      onMangaClick={() => navigate(ROUTES.manga(r.manga_id))}
                    />
                  ))}
                </div>
              </section>
            )}

            {h.recentUp.length > 0 && (
              <section className="mt-8 px-6 space-y-4">
                <h2 className="text-xl font-bold">Recently Updated</h2>
                <div className="grid grid-cols-3 gap-3">
                  {h.recentUp.map((m) => (
                    <RecentCard key={m.id} manga={m} onClick={() => navigate(ROUTES.manga(m.id))} />
                  ))}
                </div>
              </section>
            )}

            {(h.trendingLoading || h.trending.length >= 2) && (
              <section className="mt-8 px-6 space-y-4">
                <h2 className="text-xl font-bold">Trending Now</h2>
                <div className="flex gap-4 overflow-x-auto pb-2 -mx-6 px-6 [&::-webkit-scrollbar]:hidden [-ms-overflow-style:none] [scrollbar-width:none]">
                  {h.trendingLoading
                    ? Array.from({ length: 6 }).map((_, i) => (
                        <div key={i} className="shrink-0 w-36">
                          <div className="rounded-xl aspect-[2/3] bg-muted animate-pulse" />
                          <div className="mt-2 h-3 w-24 bg-muted animate-pulse rounded" />
                        </div>
                      ))
                    : h.trending.map((r, i) => (
                        <TrendingCard
                          key={r.mangaupdates_id}
                          result={r}
                          badge={['HOT', 'TOP', 'NEW', '🔥', '📈', '⭐', '💥', '🎯'][i] ?? '•'}
                          onClick={() => h.setSelectedTrending(r)}
                        />
                      ))
                  }
                </div>
              </section>
            )}
          </div>
        )}
      </div>

      <button
        onClick={() => navigate(ROUTES.discover)}
        title="Discover manga"
        className="fixed bottom-6 right-6 w-12 h-12 rounded-full bg-primary text-primary-foreground flex items-center justify-center shadow-xl hover:opacity-90 transition-opacity z-10"
      >
        <Plus className="w-5 h-5" />
      </button>

      {h.selectedTrending && (
        <TrendingModal
          result={h.selectedTrending}
          onClose={() => h.setSelectedTrending(null)}
          onViewDetails={(id) => { h.setSelectedTrending(null); navigate(ROUTES.manga(id)) }}
        />
      )}
    </>
  )
}
