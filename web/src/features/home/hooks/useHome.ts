import { useState, useEffect, useRef } from 'react'
import { api, getAllowExplicit, type SearchResult, type NewReleaseItem, type ContinueItem } from '@/api'
import type { Title } from '@/types'

export interface HomeHandle {
  items: Title[]
  isLoading: boolean
  trending: SearchResult[]
  trendingLoading: boolean
  trendingManga: SearchResult[]
  trendingMangaLoading: boolean
  trendingManhwa: SearchResult[]
  trendingManhwaLoading: boolean
  trendingManhua: SearchResult[]
  trendingManhuaLoading: boolean
  trendingAdultManhwa: SearchResult[]
  trendingAdultManhwaLoading: boolean
  newReleases: NewReleaseItem[]
  continueItems: ContinueItem[]
  recentUp: Title[]
  totalRead: number
  coverManga: { id: string; cover_url: string | null } | null
  selectedTrending: SearchResult | null
  setSelectedTrending: (r: SearchResult | null) => void
  refreshLibrary: () => void
}

export function useHome(): HomeHandle {
  const [selectedTrending, setSelectedTrending] = useState<SearchResult | null>(null)

  const [mangaData, setMangaData] = useState<{ items: Title[]; total: number; limit: number } | undefined>()
  const [isLoading, setIsLoading] = useState(true)

  const [trendingMangaData, setTrendingMangaData] = useState<SearchResult[]>([])
  const [trendingMangaLoading, setTrendingMangaLoading] = useState(true)
  const [trendingManhwaData, setTrendingManhwaData] = useState<SearchResult[]>([])
  const [trendingManhwaLoading, setTrendingManhwaLoading] = useState(true)
  const [trendingManhuaData, setTrendingManhuaData] = useState<SearchResult[]>([])
  const [trendingManhuaLoading, setTrendingManhuaLoading] = useState(true)
  const [trendingAdultManhwaData, setTrendingAdultManhwaData] = useState<SearchResult[]>([])
  const [trendingAdultManhwaLoading, setTrendingAdultManhwaLoading] = useState(true)

  const [newReleasesData, setNewReleasesData] = useState<NewReleaseItem[]>([])
  const [continueData, setContinueData] = useState<ContinueItem[]>([])

  const allowExplicit = getAllowExplicit()

  // Manga list — initial fetch
  useEffect(() => {
    api.listTitles(1)
      .then(setMangaData)
      .catch(() => {})
      .finally(() => setIsLoading(false))
  }, [])

  // Trending manga lane — MU
  useEffect(() => {
    setTrendingMangaLoading(true)
    api.getTrendingManga()
      .then(setTrendingMangaData)
      .catch(() => {})
      .finally(() => setTrendingMangaLoading(false))
  }, [])

  // Trending manhwa lane — AniList KR
  useEffect(() => {
    setTrendingManhwaLoading(true)
    api.getTrendingManhwa()
      .then(setTrendingManhwaData)
      .catch(() => {})
      .finally(() => setTrendingManhwaLoading(false))
  }, [])

  // Trending manhua lane — AniList CN
  useEffect(() => {
    setTrendingManhuaLoading(true)
    api.getTrendingManhua()
      .then(setTrendingManhuaData)
      .catch(() => {})
      .finally(() => setTrendingManhuaLoading(false))
  }, [])

  // Trending adult manhwa lane — AniList KR isAdult (only when allow_explicit)
  useEffect(() => {
    if (!allowExplicit) {
      setTrendingAdultManhwaLoading(false)
      return
    }
    setTrendingAdultManhwaLoading(true)
    api.getTrendingAdultManhwa()
      .then(setTrendingAdultManhwaData)
      .catch(() => {})
      .finally(() => setTrendingAdultManhwaLoading(false))
  }, [allowExplicit])

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
        api.listTitles(1).then(setMangaData).catch(() => {})
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

  const trendingManga = trendingMangaData.filter((r) => !r.in_library)
  const trendingManhwa = trendingManhwaData.filter((r) => !r.in_library)
  const trendingManhua = trendingManhuaData.filter((r) => !r.in_library)
  const trendingAdultManhwa = trendingAdultManhwaData.filter((r) => !r.in_library)

  // Legacy single-lane compat (uses manga lane)
  const trending = trendingManga.slice(0, 8)
  const trendingLoading = trendingMangaLoading

  const newReleases = newReleasesData ?? []
  const continueItems = continueData ?? []
  const totalRead = items.reduce((s, m) => s + (m.chapters_read ?? 0), 0)
  const coverManga =
    continueItems.length > 0
      ? { id: continueItems[0].title_id, cover_url: continueItems[0].cover_url }
      : items[0] ?? null

  function refreshLibrary() {
    api.listTitles(1).then(setMangaData).catch(() => {})
  }

  return {
    items,
    isLoading,
    trending,
    trendingLoading,
    trendingManga,
    trendingMangaLoading,
    trendingManhwa,
    trendingManhwaLoading,
    trendingManhua,
    trendingManhuaLoading,
    trendingAdultManhwa,
    trendingAdultManhwaLoading,
    newReleases,
    continueItems,
    recentUp,
    totalRead,
    coverManga,
    selectedTrending,
    setSelectedTrending,
    refreshLibrary,
  }
}
