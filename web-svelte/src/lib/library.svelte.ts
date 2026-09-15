import { api } from './api'
import type { PaginatedTitle } from './types'

export type SortOption = 'recent' | 'title_asc' | 'title_desc' | 'year'

export class LibraryStore {
  search = $state('')
  page = $state(1)
  data = $state<PaginatedTitle | undefined>(undefined)
  loading = $state(true)
  removingId = $state<string | null>(null)
  syncMessages = $state<Record<string, string>>({})
  sort = $state<SortOption>('recent')
  contentTypes = $state<string[]>([])
  statuses = $state<string[]>([])
  showFilters = $state(false)

  get hasFilters() {
    return this.contentTypes.length > 0 || this.statuses.length > 0
  }

  get totalPages() {
    return this.data ? Math.ceil(this.data.total / this.data.limit) : 1
  }

  constructor() {
    $effect(() => {
      // Reading these fields registers them as this effect's dependencies.
      void this.page
      void this.search
      void this.sort
      void this.contentTypes
      void this.statuses
      this.loading = true
      this.fetchData()
    })

    $effect(() => {
      const id = setInterval(() => this.#poll(), 2000)
      return () => clearInterval(id)
    })
  }

  fetchData = () => {
    api
      .listTitles(
        this.page,
        this.search || undefined,
        this.sort,
        this.contentTypes.length ? this.contentTypes : undefined,
        this.statuses.length ? this.statuses : undefined,
      )
      .then((d) => (this.data = d))
      .catch(() => {})
      .finally(() => (this.loading = false))
  }

  async #poll() {
    const syncingIds = (this.data?.items ?? [])
      .filter((m) => m.sync_status === 'syncing')
      .map((m) => m.id)
    if (syncingIds.length === 0) return

    this.fetchData()
    const results = await Promise.allSettled(syncingIds.map((id) => api.getSyncLog(id)))
    const next = { ...this.syncMessages }
    results.forEach((r, i) => {
      if (r.status === 'fulfilled' && r.value.length > 0) {
        next[syncingIds[i]] = r.value[r.value.length - 1].message
      }
    })
    this.syncMessages = next
  }

  setPage(value: number | ((p: number) => number)) {
    this.page = typeof value === 'function' ? value(this.page) : value
  }

  setSort(v: SortOption) {
    this.sort = v
    this.setPage(1)
  }

  toggleContentType(v: string) {
    this.contentTypes = this.contentTypes.includes(v)
      ? this.contentTypes.filter((x) => x !== v)
      : [...this.contentTypes, v]
    this.setPage(1)
  }

  toggleStatus(v: string) {
    this.statuses = this.statuses.includes(v) ? this.statuses.filter((x) => x !== v) : [...this.statuses, v]
    this.setPage(1)
  }

  clearFilters() {
    this.contentTypes = []
    this.statuses = []
    this.setPage(1)
  }

  async handleRemove(id: string, deleteFiles: boolean) {
    this.removingId = id
    try {
      await api.removeTitle(id, deleteFiles)
      this.fetchData()
    } catch {
      // ignore
    } finally {
      this.removingId = null
    }
  }
}
