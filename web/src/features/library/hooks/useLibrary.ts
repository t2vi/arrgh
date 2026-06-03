import { useState, useEffect, useRef, useCallback } from 'react'
import { api } from '@/api'
import type { PaginatedTitle } from '@/types'

export type SortOption = 'recent' | 'title_asc' | 'title_desc' | 'year'

export interface LibraryHandle {
  search: string
  setSearch: (v: string) => void
  page: number
  setPage: (fn: (p: number) => number) => void
  data: PaginatedTitle | undefined
  loading: boolean
  removingId: string | null
  totalPages: number
  syncMessages: Record<string, string>
  handleRemove: (id: string, deleteFiles: boolean) => void
  sort: SortOption
  setSort: (v: SortOption) => void
  contentTypes: string[]
  toggleContentType: (v: string) => void
  statuses: string[]
  toggleStatus: (v: string) => void
  hasFilters: boolean
  clearFilters: () => void
  showFilters: boolean
  setShowFilters: (v: boolean) => void
}

export function useLibrary(): LibraryHandle {
  const [search, setSearch] = useState('')
  const [page, setPage] = useState(1)
  const [data, setData] = useState<PaginatedTitle | undefined>()
  const [loading, setLoading] = useState(true)
  const [removingId, setRemovingId] = useState<string | null>(null)
  const [syncMessages, setSyncMessages] = useState<Record<string, string>>({})
  const [sort, _setSort] = useState<SortOption>('recent')
  const [contentTypes, setContentTypes] = useState<string[]>([])
  const [statuses, setStatuses] = useState<string[]>([])
  const [showFilters, setShowFilters] = useState(false)
  const intervalRef = useRef<ReturnType<typeof setInterval> | null>(null)

  const setSort = useCallback((v: SortOption) => { _setSort(v); setPage(() => 1) }, [])

  const toggleContentType = useCallback((v: string) => {
    setContentTypes((prev) => prev.includes(v) ? prev.filter((x) => x !== v) : [...prev, v])
    setPage(() => 1)
  }, [])

  const toggleStatus = useCallback((v: string) => {
    setStatuses((prev) => prev.includes(v) ? prev.filter((x) => x !== v) : [...prev, v])
    setPage(() => 1)
  }, [])

  const clearFilters = useCallback(() => {
    setContentTypes([])
    setStatuses([])
    setPage(() => 1)
  }, [])

  const hasFilters = contentTypes.length > 0 || statuses.length > 0

  const fetchData = useCallback(() => {
    api.listTitles(page, search || undefined, sort, contentTypes.length ? contentTypes : undefined, statuses.length ? statuses : undefined)
      .then(setData)
      .catch(() => {})
      .finally(() => setLoading(false))
  }, [page, search, sort, contentTypes, statuses])

  useEffect(() => {
    setLoading(true)
    fetchData()
  }, [fetchData])

  useEffect(() => {
    if (intervalRef.current) clearInterval(intervalRef.current)
    intervalRef.current = setInterval(async () => {
      const syncingIds = data?.items.filter((m) => m.sync_status === 'syncing').map((m) => m.id) ?? []
      if (syncingIds.length === 0) return
      api.listTitles(page, search || undefined, sort, contentTypes.length ? contentTypes : undefined, statuses.length ? statuses : undefined).then(setData).catch(() => {})
      const results = await Promise.allSettled(syncingIds.map((id) => api.getSyncLog(id)))
      setSyncMessages((prev) => {
        const next = { ...prev }
        results.forEach((r, i) => {
          if (r.status === 'fulfilled' && r.value.length > 0) {
            next[syncingIds[i]] = r.value[r.value.length - 1].message
          }
        })
        return next
      })
    }, 2000)
    return () => {
      if (intervalRef.current) clearInterval(intervalRef.current)
    }
  }, [data, page, search, sort, contentTypes, statuses])

  async function handleRemove(id: string, deleteFiles: boolean) {
    setRemovingId(id)
    try {
      await api.removeTitle(id, deleteFiles)
      fetchData()
    } catch {
      // ignore
    } finally {
      setRemovingId(null)
    }
  }

  const totalPages = data ? Math.ceil(data.total / data.limit) : 1

  return {
    search, setSearch, page, setPage, data, loading, removingId, totalPages, syncMessages, handleRemove,
    sort, setSort, contentTypes, toggleContentType, statuses, toggleStatus, hasFilters, clearFilters,
    showFilters, setShowFilters,
  }
}
