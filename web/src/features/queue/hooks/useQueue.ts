import { useState, useEffect, useRef, useCallback } from 'react'
import { api, type QueueItem } from '@/api'

const STATUS_ORDER: Record<string, number> = {
  downloading: 0, pending: 1, done: 2, error: 3, cancelled: 4,
}

function isActive(item: QueueItem) {
  return item.status === 'pending' || item.status === 'downloading'
}

export interface QueueHandle {
  data: QueueItem[] | undefined
  loading: boolean
  clearingCompleted: boolean
  canClear: boolean
  handleRemove: (itemId: string) => void
  handleClearCompleted: () => void
}

export function useQueue(): QueueHandle {
  const hadActiveRef = useRef(false)
  const [raw, setRaw] = useState<QueueItem[] | undefined>()
  const [loading, setLoading] = useState(true)
  const [clearingCompleted, setClearingCompleted] = useState(false)

  const fetchQueue = useCallback(() => {
    api.getQueue()
      .then(setRaw)
      .catch(() => {})
      .finally(() => setLoading(false))
  }, [])

  useEffect(() => { fetchQueue() }, [fetchQueue])

  useEffect(() => {
    const id = setInterval(fetchQueue, 2000)
    return () => clearInterval(id)
  }, [fetchQueue])

  const data = raw?.slice().sort(
    (a, b) => (STATUS_ORDER[a.status] ?? 9) - (STATUS_ORDER[b.status] ?? 9)
  )

  async function handleRemove(itemId: string) {
    await api.removeFromQueue(itemId).catch(() => {})
    fetchQueue()
  }

  async function handleClearCompleted() {
    setClearingCompleted(true)
    try {
      await api.clearCompletedQueue()
      fetchQueue()
    } catch {
      // ignore
    } finally {
      setClearingCompleted(false)
    }
  }

  useEffect(() => {
    if (!data) return
    const hasActive = data.some(isActive)
    const hasFinished = data.some((i) => i.status === 'done' || i.status === 'cancelled')
    if (hadActiveRef.current && !hasActive && hasFinished) {
      handleClearCompleted()
    }
    hadActiveRef.current = hasActive
  }, [data])

  const hasCompletedItems = data?.some((i) => !isActive(i) && i.status !== 'error') ?? false
  const hasErrorItems = data?.some((i) => i.status === 'error') ?? false
  const canClear = hasCompletedItems || hasErrorItems

  return { data, loading, clearingCompleted, canClear, handleRemove, handleClearCompleted }
}
