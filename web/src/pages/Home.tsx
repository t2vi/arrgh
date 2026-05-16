import { useState } from 'react'
import { useQuery, useMutation, useQueryClient } from '@tanstack/react-query'
import { useNavigate } from 'react-router-dom'
import { Play, Plus, X, BookOpen, CheckCircle2, Loader2, Download } from 'lucide-react'
import { api, getUsername, type SearchResult, type NewReleaseItem, type ContinueItem } from '@/api'
import type { Manga } from '@/types'
import { Button } from '@/components/ui/button'
import { Skeleton } from '@/components/ui/skeleton'
import { queryKeys } from '@/lib/queryKeys'
import { ROUTES } from '@/lib/routes'
import { useHome } from '@/features/home/hooks/useHome'

function timeAgo(iso: string): string {
  const diff = Date.now() - new Date(iso).getTime()
  const mins = Math.floor(diff / 60000)
  if (mins < 1) return 'Just now'
  if (mins < 60) return `${mins}m ago`
  const hrs = Math.floor(mins / 60)
  if (hrs < 24) return `${hrs} hour${hrs !== 1 ? 's' : ''} ago`
  const days = Math.floor(hrs / 24)
  if (days === 1) return 'Yesterday'
  return `${days} days ago`
}

function imgSrc(manga: Manga): string {
  return !manga.cover_url?.startsWith('http') ? api.coverUrl(manga.id) : manga.cover_url
}

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
            <p className="text-muted-foreground text-sm">Library empty — discover manga to add some.</p>
            <Button onClick={() => navigate(ROUTES.discover)}>Discover Manga</Button>
          </div>
        ) : (
          <div className="pb-12">
            <GreetingJumbotron
              totalManga={h.items.length}
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

            {h.trending.length >= 2 && (
              <section className="mt-8 px-6 space-y-4">
                <h2 className="text-xl font-bold">Trending Now</h2>
                <div className="flex gap-4 overflow-x-auto pb-2 -mx-6 px-6 [&::-webkit-scrollbar]:hidden [-ms-overflow-style:none] [scrollbar-width:none]">
                  {h.trending.map((r, i) => (
                    <TrendingCard
                      key={r.id}
                      result={r}
                      badge={['HOT', 'TOP', 'NEW', '🔥', '📈', '⭐', '💥', '🎯'][i] ?? '•'}
                      onClick={() => h.setSelectedTrending(r)}
                    />
                  ))}
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

// ——— Greeting jumbotron ———

function greeting(): string {
  const hour = new Date().getHours()
  if (hour < 12) return 'Good morning'
  if (hour < 17) return 'Good afternoon'
  return 'Good evening'
}

function GreetingJumbotron({
  totalManga,
  totalRead,
  coverManga,
}: {
  totalManga: number
  totalRead: number
  coverManga: { id: string; cover_url: string | null } | null
}) {
  const [failed, setFailed] = useState(false)
  const username = getUsername()
  const coverSrc = !failed && coverManga
    ? (coverManga.cover_url?.startsWith('http') ? coverManga.cover_url : api.coverUrl(coverManga.id))
    : null

  return (
    <div className="relative overflow-hidden" style={{ minHeight: 172 }}>
      {coverSrc && (
        <img
          src={coverSrc}
          alt=""
          className="absolute inset-0 w-full h-full object-cover object-top scale-110"
          style={{ filter: 'blur(32px)', opacity: 0.18 }}
          onError={() => setFailed(true)}
        />
      )}
      <div className="absolute inset-0 bg-gradient-to-b from-background/60 via-background/80 to-background" />
      <div className="absolute inset-0 bg-gradient-to-r from-primary/5 to-transparent" />
      <div className="relative z-10 px-8 pt-10 pb-8">
        <p className="text-[11px] font-bold uppercase tracking-[0.25em] text-primary mb-2">*ARRgh</p>
        <h1 className="text-4xl font-extrabold tracking-tight leading-none mb-3">
          {greeting()}{username ? `, ${username}` : ''}.
        </h1>
        <div className="flex items-center gap-3 text-sm text-muted-foreground flex-wrap">
          {totalManga > 0 && (
            <span className="flex items-center gap-1.5">
              <span className="w-1 h-1 rounded-full bg-primary inline-block" />
              <span><span className="font-semibold text-foreground">{totalManga}</span> manga in library</span>
            </span>
          )}
          {totalRead > 0 && (
            <span className="flex items-center gap-1.5">
              <span className="w-1 h-1 rounded-full bg-muted-foreground inline-block" />
              <span><span className="font-semibold text-foreground">{totalRead}</span> chapters read</span>
            </span>
          )}
          {totalManga === 0 && (
            <span className="text-muted-foreground">Your library is empty — discover some manga to get started.</span>
          )}
        </div>
      </div>
    </div>
  )
}

// ——— Library card ———

function LibraryCoverCard({ manga, onClick }: { manga: Manga; onClick: () => void }) {
  const [failed, setFailed] = useState(false)
  const src = !failed ? imgSrc(manga) : ''
  const tags = manga.tags?.split(', ').slice(0, 2).join(', ') ?? ''

  return (
    <div
      data-nav
      tabIndex={0}
      className="shrink-0 w-36 cursor-pointer group"
      onClick={onClick}
      onKeyDown={(e) => { if (e.key === 'Enter' || e.key === ' ') onClick() }}
    >
      <div className="relative rounded-xl overflow-hidden transition-all duration-300 group-hover:scale-[1.04] group-hover:shadow-[0_0_20px_rgba(139,92,246,0.35)]">
        {failed || !src ? (
          <div className="w-full aspect-[2/3] bg-muted flex items-center justify-center text-3xl">📖</div>
        ) : (
          <img src={src} alt={manga.title} className="w-full aspect-[2/3] object-cover bg-muted block" onError={() => setFailed(true)} />
        )}
        <div className="absolute inset-x-0 bottom-0 h-1/3 bg-gradient-to-t from-black/70 to-transparent opacity-0 group-hover:opacity-100 transition-opacity pointer-events-none" />
        {manga.sync_status === 'syncing' && (
          <div className="absolute inset-0 flex items-center justify-center bg-black/40">
            <Loader2 className="w-5 h-5 animate-spin text-primary" />
          </div>
        )}
      </div>
      <div className="mt-2 px-0.5">
        <p className="text-sm font-semibold line-clamp-1">{manga.title}</p>
        {tags && <p className="text-[11px] text-muted-foreground truncate mt-0.5">{tags}</p>}
      </div>
    </div>
  )
}

// ——— Recent card ———

function RecentCard({ manga, onClick }: { manga: Manga; onClick: () => void }) {
  const [failed, setFailed] = useState(false)
  const src = !failed ? imgSrc(manga) : ''

  return (
    <div
      data-nav
      tabIndex={0}
      className="flex gap-3 rounded-lg bg-card border border-border p-3 cursor-pointer hover:bg-accent/40 transition-colors"
      onClick={onClick}
      onKeyDown={(e) => { if (e.key === 'Enter' || e.key === ' ') onClick() }}
    >
      <div className="shrink-0 w-11">
        {failed || !src ? (
          <div className="w-full aspect-[2/3] bg-muted rounded text-lg flex items-center justify-center">📖</div>
        ) : (
          <img src={src} alt="" className="w-full aspect-[2/3] object-cover bg-muted rounded" onError={() => setFailed(true)} />
        )}
      </div>
      <div className="min-w-0 flex-1">
        <p className="text-sm font-semibold truncate">{manga.title}</p>
        {(manga.total_chapters ?? 0) > 0 && (
          <p className="text-[11px] text-muted-foreground">Chapter {manga.total_chapters} available</p>
        )}
        <p className="text-[11px] text-muted-foreground mt-0.5">{timeAgo(manga.updated_at)}</p>
      </div>
    </div>
  )
}

// ——— New release card ———

function NewReleaseCard({ item, onClick, onMangaClick }: {
  item: NewReleaseItem
  onClick: () => void
  onMangaClick: () => void
}) {
  const [failed, setFailed] = useState(false)
  const src = !failed
    ? (item.cover_url?.startsWith('http') ? item.cover_url : item.cover_url ? api.coverUrl(item.manga_id) : '')
    : ''
  const chNum = Number.isInteger(item.chapter_number) ? item.chapter_number : item.chapter_number.toFixed(1)

  return (
    <div className="shrink-0 w-32 flex flex-col gap-2">
      <div
        data-nav
        tabIndex={0}
        className="relative rounded-xl overflow-hidden cursor-pointer group transition-all duration-300 hover:scale-[1.04] hover:shadow-[0_0_18px_rgba(139,92,246,0.35)]"
        onClick={onClick}
        onKeyDown={(e) => { if (e.key === 'Enter' || e.key === ' ') onClick() }}
      >
        {!src ? (
          <div className="w-full aspect-[2/3] bg-muted flex items-center justify-center text-2xl">📖</div>
        ) : (
          <img src={src} alt="" className="w-full aspect-[2/3] object-cover bg-muted block" onError={() => setFailed(true)} />
        )}
        <div className="absolute top-2 left-2 px-1.5 py-0.5 rounded-md bg-primary text-primary-foreground text-[10px] font-black leading-none">
          Ch.{chNum}
        </div>
        {item.downloaded && (
          <div className="absolute top-2 right-2 p-0.5 rounded-full bg-black/50">
            <Download className="w-2.5 h-2.5 text-emerald-400" />
          </div>
        )}
        <div className="absolute inset-0 bg-black/0 group-hover:bg-black/20 flex items-center justify-center transition-colors">
          <Play className="w-6 h-6 text-white opacity-0 group-hover:opacity-100 transition-opacity fill-current drop-shadow" />
        </div>
      </div>
      <button onClick={onMangaClick} className="text-left px-0.5">
        <p className="text-xs font-semibold line-clamp-1 hover:text-primary transition-colors">{item.manga_title}</p>
        <p className="text-[10px] text-muted-foreground mt-0.5">{timeAgo(item.chapter_created_at)}</p>
      </button>
    </div>
  )
}

// ——— Trending card ———

function TrendingCard({ result, badge, onClick }: {
  result: SearchResult
  badge: string
  onClick: () => void
}) {
  const [failed, setFailed] = useState(false)
  const src = failed ? '' : (result.cover_url ? api.proxyImageUrl(result.cover_url) : '')

  return (
    <div
      data-nav
      tabIndex={0}
      className="shrink-0 w-36 cursor-pointer group"
      onClick={onClick}
      onKeyDown={(e) => { if (e.key === 'Enter' || e.key === ' ') onClick() }}
    >
      <div className="relative rounded-xl overflow-hidden aspect-[2/3] transition-all duration-300 group-hover:scale-[1.04] group-hover:shadow-[0_0_20px_rgba(139,92,246,0.35)]">
        {!src ? (
          <div className="w-full h-full bg-muted flex items-center justify-center text-3xl">📖</div>
        ) : (
          <img src={src} alt="" className="w-full h-full object-cover object-top" onError={() => setFailed(true)} />
        )}
        <div className="absolute top-2 left-2 px-1.5 py-0.5 rounded bg-primary text-primary-foreground text-[9px] font-black leading-none">
          {badge}
        </div>
        {result.in_library && (
          <div className="absolute top-2 right-2 w-2 h-2 rounded-full bg-emerald-400 shadow" />
        )}
      </div>
      <div className="mt-2 px-0.5">
        <p className="text-sm font-semibold line-clamp-1">{result.title}</p>
        {result.in_library && (
          <p className="text-[11px] text-primary mt-0.5 font-medium">In library</p>
        )}
      </div>
    </div>
  )
}

// ——— Trending modal ———

function TrendingModal({
  result,
  onClose,
  onViewDetails,
}: {
  result: SearchResult
  onClose: () => void
  onViewDetails: (libraryId: string) => void
}) {
  const queryClient = useQueryClient()
  const [coverFailed, setCoverFailed] = useState(false)
  const proxied = result.cover_url ? api.proxyImageUrl(result.cover_url) : ''

  const { data: detail, isLoading: detailLoading } = useQuery({
    queryKey: queryKeys.discoverDetail(result.source, result.id),
    queryFn: () => api.getDiscoverDetail(result.source, result.id),
    staleTime: 10 * 60 * 1000,
  })

  const addMutation = useMutation({
    mutationFn: () => api.addManga({ ...result, description: detail?.description ?? result.description }),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: queryKeys.trending() })
      queryClient.invalidateQueries({ queryKey: queryKeys.manga.all() })
    },
  })

  const coverUrl = detail?.cover_url ? api.proxyImageUrl(detail.cover_url) : proxied
  const tags = result.tags?.split(', ').filter(Boolean) ?? []
  const chapterCount = detail?.chapter_count ?? 0
  const description = detail?.description ?? result.description

  return (
    <div
      className="fixed inset-0 z-50 flex items-end sm:items-center justify-center p-4 bg-black/60 backdrop-blur-sm"
      onClick={(e) => e.target === e.currentTarget && onClose()}
    >
      <div className="relative w-full max-w-lg bg-card rounded-2xl overflow-hidden shadow-2xl border border-border animate-in fade-in slide-in-from-bottom-4 duration-200">
        <button
          onClick={onClose}
          className="absolute top-3 right-3 z-10 p-1.5 rounded-full bg-black/40 text-white/70 hover:text-white hover:bg-black/60 transition-colors"
        >
          <X className="w-4 h-4" />
        </button>

        <div className="relative h-48 overflow-hidden">
          {coverFailed || !coverUrl ? (
            <div className="w-full h-full bg-muted flex items-center justify-center text-6xl">📖</div>
          ) : (
            <img src={coverUrl} alt="" className="w-full h-full object-cover object-top" onError={() => setCoverFailed(true)} />
          )}
          <div className="absolute inset-0 bg-gradient-to-t from-card via-card/30 to-transparent" />
          <div className="absolute bottom-0 left-0 right-0 px-5 pb-4">
            <h2 className="text-xl font-extrabold leading-tight text-white drop-shadow">{result.title}</h2>
          </div>
        </div>

        <div className="px-5 pt-3 pb-5 space-y-4">
          <div className="flex items-center gap-3 flex-wrap">
            {result.status && result.status !== 'unknown' && (
              <span className="text-[11px] font-semibold px-2 py-0.5 rounded-full bg-primary/15 text-primary border border-primary/20">
                {result.status}
              </span>
            )}
            {result.in_library && (
              <span className="flex items-center gap-1 text-[11px] font-semibold text-emerald-400">
                <CheckCircle2 className="w-3.5 h-3.5" /> In library
              </span>
            )}
            {detailLoading ? (
              <span className="text-[11px] text-muted-foreground animate-pulse">Loading…</span>
            ) : chapterCount > 0 ? (
              <span className="flex items-center gap-1 text-[11px] text-muted-foreground">
                <BookOpen className="w-3.5 h-3.5" />
                {chapterCount} chapter{chapterCount !== 1 ? 's' : ''}
              </span>
            ) : null}
          </div>

          {detailLoading ? (
            <div className="space-y-1.5">
              <Skeleton className="h-3.5 w-full" />
              <Skeleton className="h-3.5 w-5/6" />
              <Skeleton className="h-3.5 w-3/4" />
            </div>
          ) : description ? (
            <p className="text-sm text-muted-foreground leading-relaxed line-clamp-4">{description}</p>
          ) : (
            <p className="text-sm text-muted-foreground italic">No description available.</p>
          )}

          {tags.length > 0 && (
            <div className="flex flex-wrap gap-1.5">
              {tags.slice(0, 6).map((tag) => (
                <span key={tag} className="text-[10px] px-2 py-0.5 rounded-full bg-muted text-muted-foreground border border-border">
                  {tag}
                </span>
              ))}
            </div>
          )}

          <div className="flex gap-2 pt-1">
            {result.in_library && result.library_id ? (
              <Button className="flex-1" onClick={() => onViewDetails(result.library_id!)}>
                View Details
              </Button>
            ) : addMutation.isSuccess ? (
              <Button className="flex-1" variant="outline" disabled>
                <CheckCircle2 className="w-4 h-4 mr-2 text-emerald-400" /> Added to Library
              </Button>
            ) : (
              <Button className="flex-1" onClick={() => addMutation.mutate()} disabled={addMutation.isPending}>
                {addMutation.isPending
                  ? <><Loader2 className="w-4 h-4 mr-2 animate-spin" /> Adding…</>
                  : <><Plus className="w-4 h-4 mr-2" /> Add to Library</>}
              </Button>
            )}
            <Button variant="outline" onClick={onClose} className="px-4">Close</Button>
          </div>
        </div>
      </div>
    </div>
  )
}

// ——— Continue Reading card ———

function ContinueCard({ item, onPlay, onDetail }: {
  item: ContinueItem
  onPlay: () => void
  onDetail: () => void
}) {
  const [failed, setFailed] = useState(false)
  const src = !failed
    ? (item.cover_url?.startsWith('http') ? item.cover_url : item.cover_url ? api.coverUrl(item.manga_id) : '')
    : ''
  const pct = item.total_chapters > 0
    ? Math.round((item.chapters_read / item.total_chapters) * 100)
    : 0

  return (
    <div
      data-nav
      tabIndex={0}
      className="shrink-0 w-36 flex flex-col gap-2 cursor-pointer group"
      onClick={onDetail}
      onKeyDown={(e) => { if (e.key === 'Enter' || e.key === ' ') onDetail() }}
    >
      <div className="relative rounded-xl overflow-hidden aspect-[2/3] transition-all duration-300 group-hover:scale-[1.04] group-hover:shadow-[0_0_20px_rgba(139,92,246,0.35)]">
        {!src ? (
          <div className="w-full h-full bg-muted flex items-center justify-center text-3xl">📖</div>
        ) : (
          <img src={src} alt="" className="w-full h-full object-cover bg-muted" onError={() => setFailed(true)} />
        )}
        <div className="absolute bottom-0 left-0 right-0 h-1 bg-black/40">
          <div className="h-full bg-primary transition-all" style={{ width: `${pct}%` }} />
        </div>
        <button
          onClick={(e) => { e.stopPropagation(); onPlay() }}
          className="absolute inset-0 flex items-center justify-center bg-black/0 group-hover:bg-black/30 transition-colors"
        >
          <div className="w-10 h-10 rounded-full bg-primary/80 flex items-center justify-center opacity-0 group-hover:opacity-100 transition-opacity shadow-lg">
            <Play className="w-4 h-4 fill-white text-white ml-0.5" />
          </div>
        </button>
      </div>
      <div className="px-0.5">
        <p className="text-xs font-semibold line-clamp-1">{item.manga_title}</p>
        <p className="text-[10px] text-primary font-medium mt-0.5">
          Ch. {Number.isInteger(item.chapter_number) ? item.chapter_number : item.chapter_number.toFixed(1)}
        </p>
        <p className="text-[10px] text-muted-foreground">{item.chapters_read}/{item.total_chapters} read</p>
      </div>
    </div>
  )
}

// ——— Skeleton ———

function HomeSkeleton() {
  return (
    <div className="space-y-8 pb-10">
      <Skeleton className="h-72 w-full" />
      <div className="px-6 space-y-4">
        <Skeleton className="h-6 w-32" />
        <div className="flex gap-4">
          {Array.from({ length: 5 }).map((_, i) => (
            <div key={i} className="shrink-0 w-36 space-y-2">
              <Skeleton className="w-full aspect-[2/3] rounded-xl" />
              <Skeleton className="h-3.5 w-3/4" />
              <Skeleton className="h-3 w-1/2" />
            </div>
          ))}
        </div>
      </div>
      <div className="px-6 space-y-4">
        <Skeleton className="h-6 w-44" />
        <div className="grid grid-cols-3 gap-3">
          {Array.from({ length: 3 }).map((_, i) => (
            <Skeleton key={i} className="h-20 rounded-lg" />
          ))}
        </div>
      </div>
      <div className="px-6 space-y-4">
        <Skeleton className="h-6 w-36" />
        <div className="grid grid-cols-2 gap-4">
          <Skeleton className="aspect-[16/10] rounded-xl" />
          <Skeleton className="aspect-[16/10] rounded-xl" />
        </div>
      </div>
    </div>
  )
}
