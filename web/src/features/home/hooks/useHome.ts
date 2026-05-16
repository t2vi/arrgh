import { useState } from 'react'
import { useQuery } from '@tanstack/react-query'
import { api, type SearchResult, type NewReleaseItem, type ContinueItem } from '@/api'
import type { Manga } from '@/types'
import { queryKeys } from '@/lib/queryKeys'

export interface HomeHandle {
  items: Manga[]
  isLoading: boolean
  trending: SearchResult[]
  newReleases: NewReleaseItem[]
  continueItems: ContinueItem[]
  recentUp: Manga[]
  totalRead: number
  coverManga: { id: string; cover_url: string | null } | null
  selectedTrending: SearchResult | null
  setSelectedTrending: (r: SearchResult | null) => void
}

export function useHome(): HomeHandle {
  const [selectedTrending, setSelectedTrending] = useState<SearchResult | null>(null)

  const { data, isLoading } = useQuery({
    queryKey: queryKeys.manga.list(1, ''),
    queryFn: () => api.listManga(1),
    refetchInterval: (query) => {
      const items = query.state.data?.items
      return items?.some((m) => m.sync_status === 'syncing') ? 2000 : false
    },
  })

  const { data: trendingData } = useQuery({
    queryKey: queryKeys.trending(),
    queryFn: () => api.getTrending(),
    staleTime: 0,
    refetchOnWindowFocus: true,
  })

  const { data: newReleasesData } = useQuery({
    queryKey: queryKeys.newReleases(),
    queryFn: () => api.getNewReleases(),
    refetchInterval: 5 * 60 * 1000,
  })

  const { data: continueData } = useQuery({
    queryKey: queryKeys.continueReading(),
    queryFn: () => api.getContinueReading(),
    refetchInterval: 30 * 1000,
  })

  const items = data?.items ?? []
  const recentUp = [...items]
    .sort((a, b) => new Date(b.updated_at).getTime() - new Date(a.updated_at).getTime())
    .slice(0, 3)
  const trending = (trendingData ?? []).filter((r) => !r.in_library).slice(0, 8)
  const newReleases = newReleasesData ?? []
  const continueItems = continueData ?? []
  const totalRead = items.reduce((s, m) => s + (m.chapters_read ?? 0), 0)
  const coverManga =
    continueItems.length > 0
      ? { id: continueItems[0].manga_id, cover_url: continueItems[0].cover_url }
      : items[0] ?? null

  return {
    items,
    isLoading,
    trending,
    newReleases,
    continueItems,
    recentUp,
    totalRead,
    coverManga,
    selectedTrending,
    setSelectedTrending,
  }
}
