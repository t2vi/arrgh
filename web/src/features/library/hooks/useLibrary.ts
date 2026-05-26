import { useState, useEffect, useRef, useCallback } from 'react'
import { api } from '@/api'
import type { PaginatedTitle } from '@/types'

export interface LibraryHandle {
  search: string
  setSearch: (v: string) => void
  page: number
  setPage: (fn: (p: number) => number) => void
  data: PaginatedTitle | undefined
  loading: boolean
  removingId: string | null
  totalPages: number
  handleRemove: (id: string, deleteFiles: boolean) => void
}

export function useLibrary(): LibraryHandle {
  const [search, setSearch] = useState('')
  const [page, setPage] = useState(1)
  const [data, setData] = useState<PaginatedTitle | undefined>()
  const [loading, setLoading] = useState(true)
  const [removingId, setRemovingId] = useState<string | null>(null)
  const intervalRef = useRef<ReturnType<typeof setInterval> | null>(null)

  const fetchData = useCallback(() => {
    api.listTitles(page, search || undefined)
      .then(setData)
      .catch(() => {})
      .finally(() => setLoading(false))
  }, [page, search])

  useEffect(() => {
    setLoading(true)
    fetchData()
  }, [fetchData])

  useEffect(() => {
    if (intervalRef.current) clearInterval(intervalRef.current)
    intervalRef.current = setInterval(() => {
      if (data?.items.some((m) => m.sync_status === 'syncing')) {
        api.listTitles(page, search || undefined).then(setData).catch(() => {})
      }
    }, 2000)
    return () => {
      if (intervalRef.current) clearInterval(intervalRef.current)
    }
  }, [data, page, search])

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

  return { search, setSearch, page, setPage, data, loading, removingId, totalPages, handleRemove }
}
