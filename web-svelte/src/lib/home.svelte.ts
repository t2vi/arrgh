import { api, getAllowExplicit, type SearchResult, type NewReleaseItem, type ContinueItem } from './api'
import type { Title } from './types'

export class HomeStore {
  selectedTrending = $state<SearchResult | null>(null)

  #mangaData = $state<{ items: Title[]; total: number; limit: number } | undefined>(undefined)
  isLoading = $state(true)

  #trendingMangaData = $state<SearchResult[]>([])
  trendingMangaLoading = $state(true)
  #trendingManhwaData = $state<SearchResult[]>([])
  trendingManhwaLoading = $state(true)
  #trendingManhuaData = $state<SearchResult[]>([])
  trendingManhuaLoading = $state(true)
  #trendingAdultManhwaData = $state<SearchResult[]>([])
  trendingAdultManhwaLoading = $state(true)

  newReleases = $state<NewReleaseItem[]>([])
  continueItems = $state<ContinueItem[]>([])

  get items() {
    return this.#mangaData?.items ?? []
  }

  get recentUp() {
    return [...this.items]
      .sort((a, b) => new Date(b.updated_at).getTime() - new Date(a.updated_at).getTime())
      .slice(0, 3)
  }

  get trendingManga() {
    return this.#trendingMangaData.filter((r) => !r.in_library)
  }
  get trendingManhwa() {
    return this.#trendingManhwaData.filter((r) => !r.in_library)
  }
  get trendingManhua() {
    return this.#trendingManhuaData.filter((r) => !r.in_library)
  }
  get trendingAdultManhwa() {
    return this.#trendingAdultManhwaData.filter((r) => !r.in_library)
  }

  get totalRead() {
    return this.items.reduce((s, m) => s + (m.chapters_read ?? 0), 0)
  }

  get coverManga(): { id: string; cover_url: string | null } | null {
    if (this.continueItems.length > 0) {
      return { id: this.continueItems[0].title_id, cover_url: this.continueItems[0].cover_url }
    }
    return this.items[0] ?? null
  }

  constructor() {
    const allowExplicit = getAllowExplicit()

    api
      .listTitles(1)
      .then((d) => (this.#mangaData = d))
      .catch(() => {})
      .finally(() => (this.isLoading = false))

    api
      .getTrendingManga()
      .then((d) => (this.#trendingMangaData = d))
      .catch(() => {})
      .finally(() => (this.trendingMangaLoading = false))

    api
      .getTrendingManhwa()
      .then((d) => (this.#trendingManhwaData = d))
      .catch(() => {})
      .finally(() => (this.trendingManhwaLoading = false))

    api
      .getTrendingManhua()
      .then((d) => (this.#trendingManhuaData = d))
      .catch(() => {})
      .finally(() => (this.trendingManhuaLoading = false))

    if (allowExplicit) {
      api
        .getTrendingAdultManhwa()
        .then((d) => (this.#trendingAdultManhwaData = d))
        .catch(() => {})
        .finally(() => (this.trendingAdultManhwaLoading = false))
    } else {
      this.trendingAdultManhwaLoading = false
    }

    api.getNewReleases().then((d) => (this.newReleases = d)).catch(() => {})
    $effect(() => {
      const id = setInterval(() => {
        api.getNewReleases().then((d) => (this.newReleases = d)).catch(() => {})
      }, 5 * 60 * 1000)
      return () => clearInterval(id)
    })

    api.getContinueReading().then((d) => (this.continueItems = d)).catch(() => {})
    $effect(() => {
      const id = setInterval(() => {
        api.getContinueReading().then((d) => (this.continueItems = d)).catch(() => {})
      }, 30 * 1000)
      return () => clearInterval(id)
    })

    // Re-fetch the manga list every 2s while any item is syncing.
    $effect(() => {
      const id = setInterval(() => {
        if (this.#mangaData?.items.some((m) => m.sync_status === 'syncing')) {
          api.listTitles(1).then((d) => (this.#mangaData = d)).catch(() => {})
        }
      }, 2000)
      return () => clearInterval(id)
    })
  }
}
