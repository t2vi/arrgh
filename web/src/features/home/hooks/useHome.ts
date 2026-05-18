import { useState, useEffect, useRef } from 'react'
import { api, type SearchResult, type NewReleaseItem, type ContinueItem } from '@/api'
import type { Manga } from '@/types'

export interface HomeHandle {
  items: Manga[]
  isLoading: boolean
  trending: SearchResult[]
  trendingLoading: boolean
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

  const [mangaData, setMangaData] = useState<{ items: Manga[]; total: number; limit: number } | undefined>()
  const [isLoading, setIsLoading] = useState(true)
  const [trendingData, setTrendingData] = useState<SearchResult[]>([])
  const [trendingLoading, setTrendingLoading] = useState(true)
  const [newReleasesData, setNewReleasesData] = useState<NewReleaseItem[]>([])
  const [continueData, setContinueData] = useState<ContinueItem[]>([])

  // Manga list — initial fetch
  useEffect(() => {
    api.listManga(1)
      .then(setMangaData)
      .catch(() => {})
      .finally(() => setIsLoading(false))
  }, [])

  // Trending — fetch on mount, refetch on window focus
  useEffect(() => {
    function fetchTrending() {
      setTrendingLoading(true)
      api.getTrending()
        .then(setTrendingData)
        .catch(() => {})
        .finally(() => setTrendingLoading(false))
    }
    fetchTrending()
    window.addEventListener('focus', fetchTrending)
    return () => window.removeEventListener('focus', fetchTrending)
  }, [])

  // Poll trending every 5s while any card is still missing a cover (downloads in-flight)
  const trendingPollRef = useRef<ReturnType<typeof setInterval> | null>(null)
  useEffect(() => {
    if (trendingPollRef.current) clearInterval(trendingPollRef.current)
    const missingCovers = trendingData.some((r) => !r.cover_url)
    if (!missingCovers || trendingLoading) return
    trendingPollRef.current = setInterval(() => {
      api.getTrending()
        .then((data) => {
          setTrendingData(data)
          if (data.every((r) => r.cover_url)) {
            if (trendingPollRef.current) clearInterval(trendingPollRef.current)
          }
        })
        .catch(() => {})
    }, 5000)
    return () => {
      if (trendingPollRef.current) clearInterval(trendingPollRef.current)
    }
  }, [trendingData, trendingLoading])

  // New releases — poll every 5 minutes
  useEffect(() => {
    api.getNewReleases().then(setNewReleasesData).catch(() => {})
    const id = setInterval(() => {
      api.getNewReleases().then(setNewReleasesData).catch(() => {})
    }, 5 * 60 * 1000)
    return () => clearInterval(id)
  }, [])

  // Continue reading — poll every 30s
  useEffect(() => {
    api.getContinueReading().then(setContinueData).catch(() => {})
    const id = setInterval(() => {
      api.getContinueReading().then(setContinueData).catch(() => {})
    }, 30 * 1000)
    return () => clearInterval(id)
  }, [])

  // Polling: re-fetch manga list every 2s while any are syncing
  const mangaIntervalRef = useRef<ReturnType<typeof setInterval> | null>(null)
  useEffect(() => {
    if (mangaIntervalRef.current) clearInterval(mangaIntervalRef.current)
    mangaIntervalRef.current = setInterval(() => {
      if (mangaData?.items.some((m) => m.sync_status === 'syncing')) {
        api.listManga(1).then(setMangaData).catch(() => {})
      }
    }, 2000)
    return () => {
      if (mangaIntervalRef.current) clearInterval(mangaIntervalRef.current)
    }
  }, [mangaData])

  const items = mangaData?.items ?? []
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
    trendingLoading,
    newReleases,
    continueItems,
    recentUp,
    totalRead,
    coverManga,
    selectedTrending,
    setSelectedTrending,
  }
}
