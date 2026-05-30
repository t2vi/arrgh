import { useState, useEffect, useRef, useMemo, useCallback } from 'react'
import { useNavigate } from 'react-router-dom'
import { api, type QueueItem } from '@/api'
import type { Chapter, ReadProgress, SyncLogEntry } from '@/types'
import { ROUTES } from '@/lib/routes'

export type FilterMode = 'all' | 'downloaded' | 'not_downloaded'
export type SortDir = 'desc' | 'asc'

export interface SimpleMutation<TArgs = void> {
  mutate: (args: TArgs) => void
  isPending: boolean
  isSuccess: boolean
}

export interface MangaDetailHandle {
  manga: Awaited<ReturnType<typeof api.getTitle>> | undefined
  chapters: Chapter[]
  allProgress: ReadProgress[]
  queueItems: QueueItem[]
  progressMap: Map<string, ReadProgress>
  queueMap: Map<string, QueueItem>
  loadingManga: boolean
  loadingChapters: boolean
  isSyncing: boolean
  isRemoteSource: boolean
  // Derived counts
  total: number
  downloaded: number
  streamable: number
  readCount: number
  readPct: number
  activeCount: number
  pendingCount: number
  resumeChapter: Chapter | undefined
  // Filters
  showAll: boolean
  setShowAll: (v: boolean) => void
  filterMode: FilterMode
  setFilterMode: (v: FilterMode) => void
  sortDir: SortDir
  setSortDir: (fn: (prev: SortDir) => SortDir) => void
  filteredChapters: Chapter[]
  displayed: Chapter[]
  tags: string[]
  coverSrc: string
  // UI state
  showRemoveMenu: boolean
  setShowRemoveMenu: (fn: (v: boolean) => boolean) => void
  removeMenuRef: React.RefObject<HTMLDivElement | null>
  pendingReadId: string | null
  syncLog: SyncLogEntry[]
  // Actions
  openOrQueue: (ch: Chapter) => void
  sync: SimpleMutation<void>
  refreshMetadata: SimpleMutation<void>
  removeFromQueue: SimpleMutation<string>
  cancelAll: SimpleMutation<void>
  downloadAll: SimpleMutation<void>
  refreshManga: () => void
}

const CHAPTERS_PREVIEW = 5

export function useMangaDetail(id: string | undefined): MangaDetailHandle {
  const navigate = useNavigate()

  const [showAll, setShowAll] = useState(false)
  const [filterMode, setFilterMode] = useState<FilterMode>('all')
  const [sortDir, setSortDir] = useState<SortDir>('desc')
  const [showRemoveMenu, setShowRemoveMenu] = useState(false)
  const [pendingReadId, setPendingReadId] = useState<string | null>(null)
  const removeMenuRef = useRef<HTMLDivElement>(null)

  const [manga, setManga] = useState<Awaited<ReturnType<typeof api.getTitle>> | undefined>()
  const [loadingManga, setLoadingManga] = useState(true)
  const [chapters, setChapters] = useState<Chapter[]>([])
  const [loadingChapters, setLoadingChapters] = useState(true)
  const [allProgress, setAllProgress] = useState<ReadProgress[]>([])
  const [queueItems, setQueueItems] = useState<QueueItem[]>([])
  const [syncLog, setSyncLog] = useState<SyncLogEntry[]>([])

  // Mutation states
  const [syncPending, setSyncPending] = useState(false)
  const [syncSuccess, setSyncSuccess] = useState(false)
  const [refreshMetaPending, setRefreshMetaPending] = useState(false)
  const [refreshMetaSuccess, setRefreshMetaSuccess] = useState(false)
  const [removeFromQueuePending, setRemoveFromQueuePending] = useState(false)
  const [cancelAllPending, setCancelAllPending] = useState(false)
  const [downloadAllPending, setDownloadAllPending] = useState(false)

  useEffect(() => {
    if (!showRemoveMenu) return
    const handler = (e: MouseEvent) => {
      if (removeMenuRef.current && !removeMenuRef.current.contains(e.target as Node)) {
        setShowRemoveMenu(false)
      }
    }
    document.addEventListener('mousedown', handler)
    return () => document.removeEventListener('mousedown', handler)
  }, [showRemoveMenu])

  const fetchManga = useCallback(() => {
    if (!id) return
    api.getTitle(id).then(setManga).catch(() => {}).finally(() => setLoadingManga(false))
  }, [id])

  const fetchChapters = useCallback(() => {
    if (!id) return
    api.listChapters(id).then(setChapters).catch(() => {}).finally(() => setLoadingChapters(false))
  }, [id])

  const fetchProgress = useCallback(() => {
    if (!id) return
    api.getTitleProgress(id).then(setAllProgress).catch(() => {})
  }, [id])

  const fetchQueue = useCallback(() => {
    if (!id) return
    api.getTitleQueue(id).then(setQueueItems).catch(() => {})
  }, [id])

  const fetchSyncLog = useCallback(() => {
    if (!id) return
    api.getSyncLog(id).then(setSyncLog).catch(() => {})
  }, [id])

  // Initial fetches
  useEffect(() => {
    if (!id) return
    setLoadingManga(true)
    fetchManga()
  }, [id, fetchManga])

  useEffect(() => {
    if (!manga) return
    fetchChapters()
    fetchProgress()
  }, [manga, fetchChapters, fetchProgress])

  // Poll manga every 2s while syncing
  useEffect(() => {
    if (manga?.sync_status !== 'syncing') return
    const id = setInterval(fetchManga, 2000)
    return () => clearInterval(id)
  }, [manga?.sync_status, fetchManga])

  // Poll chapters every 3s while syncing
  useEffect(() => {
    if (manga?.sync_status !== 'syncing') return
    const id = setInterval(fetchChapters, 3000)
    return () => clearInterval(id)
  }, [manga?.sync_status, fetchChapters])

  // Fetch queue on mount
  useEffect(() => {
    if (!id) return
    fetchQueue()
  }, [id, fetchQueue])

  // Fetch sync log on mount and poll every 2s while syncing
  useEffect(() => {
    if (!id) return
    fetchSyncLog()
  }, [id, fetchSyncLog])

  useEffect(() => {
    if (manga?.sync_status !== 'syncing') return
    const tid = setInterval(fetchSyncLog, 2000)
    return () => clearInterval(tid)
  }, [manga?.sync_status, fetchSyncLog])

  // Poll every 2s only while there are active items; stops automatically when idle
  useEffect(() => {
    const hasActive = queueItems.some(
      (q) => q.status === 'pending' || q.status === 'downloading',
    )
    if (!hasActive) return
    const tid = setInterval(fetchQueue, 2000)
    return () => clearInterval(tid)
  }, [queueItems, fetchQueue])

  const progressMap = new Map<string, ReadProgress>(allProgress.map((p) => [p.chapter_id, p]))
  const queueMap = new Map<string, QueueItem>(queueItems.map((q) => [q.chapter_id, q]))

  const prevQueueRef = useRef<QueueItem[]>([])
  useEffect(() => {
    const prev = prevQueueRef.current
    const justDone = queueItems.some(
      (q) => q.status === 'done' && prev.find((p) => p.id === q.id)?.status !== 'done',
    )
    if (justDone) fetchChapters()
    prevQueueRef.current = queueItems
  }, [queueItems, fetchChapters])

  useEffect(() => {
    if (!pendingReadId) return
    const qi = queueMap.get(pendingReadId)
    if (qi?.status === 'error') { setPendingReadId(null); return }
    const ch = chapters.find((c) => c.id === pendingReadId)
    if (ch?.downloaded || qi?.status === 'done') {
      setPendingReadId(null)
      navigate(ROUTES.reader(pendingReadId))
    }
  }, [pendingReadId, chapters, queueMap, navigate])

  function openOrQueue(ch: Chapter) {
    if (ch.downloaded) { navigate(ROUTES.reader(ch.id)); return }
    if (!ch.has_sources) return
    const qi = queueMap.get(ch.id)
    if (!qi || qi.status === 'error' || qi.status === 'cancelled') {
      api.downloadChapter(ch.id)
        .then(() => fetchQueue())
        .catch(() => {})
    }
    setPendingReadId(ch.id)
  }

  const sync: SimpleMutation<void> = {
    mutate: () => {
      if (!id) return
      setSyncPending(true)
      setSyncSuccess(false)
      api.syncTitle(id)
        .then(() => {
          setSyncSuccess(true)
          fetchManga()
          setTimeout(() => fetchChapters(), 2000)
        })
        .catch(() => {})
        .finally(() => setSyncPending(false))
    },
    isPending: syncPending,
    isSuccess: syncSuccess,
  }

  const refreshMetadata: SimpleMutation<void> = {
    mutate: () => {
      if (!id) return
      setRefreshMetaPending(true)
      setRefreshMetaSuccess(false)
      api.refreshMetadata(id)
        .then(() => {
          setRefreshMetaSuccess(true)
          fetchManga()
          setTimeout(() => fetchChapters(), 3000)
        })
        .catch(() => {})
        .finally(() => setRefreshMetaPending(false))
    },
    isPending: refreshMetaPending,
    isSuccess: refreshMetaSuccess,
  }

  const removeFromQueue: SimpleMutation<string> = {
    mutate: (itemId: string) => {
      setRemoveFromQueuePending(true)
      api.removeFromQueue(itemId)
        .catch(() => {})
        .finally(() => setRemoveFromQueuePending(false))
    },
    isPending: removeFromQueuePending,
    isSuccess: false,
  }

  const cancelAll: SimpleMutation<void> = {
    mutate: () => {
      setCancelAllPending(true)
      const toCancel = queueItems.filter((q) => q.status === 'pending' || q.status === 'downloading')
      Promise.all(toCancel.map((q) => api.removeFromQueue(q.id).catch(() => {})))
        .then(() => fetchQueue())
        .finally(() => setCancelAllPending(false))
    },
    isPending: cancelAllPending,
    isSuccess: false,
  }

  const downloadAll: SimpleMutation<void> = {
    mutate: () => {
      setDownloadAllPending(true)
      const toDownload = chapters.filter((c) => !c.downloaded && c.has_sources)
      Promise.all(toDownload.map((c) => api.downloadChapter(c.id).catch(() => {})))
        .then(() => fetchQueue())
        .finally(() => setDownloadAllPending(false))
    },
    isPending: downloadAllPending,
    isSuccess: false,
  }

  const isRemoteSource = manga != null && !manga.is_local
  const isSyncing = manga?.sync_status === 'syncing'

  const total = chapters.length
  const downloaded = chapters.filter((c) => c.downloaded).length
  const streamable = chapters.filter((c) => !c.downloaded && c.has_sources).length
  const readCount = allProgress.filter((p) => p.completed).length
  const readPct = total > 0 ? Math.round((readCount / total) * 100) : 0
  const activeCount = queueItems.filter((q) => q.status === 'downloading').length
  const pendingCount = queueItems.filter((q) => q.status === 'pending').length

  const resumeChapter = (() => {
    const inProgress = chapters.find((c) => {
      const p = progressMap.get(c.id)
      return p && !p.completed
    })
    if (inProgress) return inProgress
    return [...chapters].sort((a, b) => a.number - b.number).find((c) => !progressMap.get(c.id)?.completed)
  })()

  const filteredChapters = useMemo(() => {
    let result = [...chapters]
    if (filterMode === 'downloaded') result = result.filter((c) => c.downloaded)
    else if (filterMode === 'not_downloaded') result = result.filter((c) => !c.downloaded)
    result.sort((a, b) => sortDir === 'asc' ? a.number - b.number : b.number - a.number)
    return result
  }, [chapters, filterMode, sortDir])

  const displayed = showAll ? filteredChapters : filteredChapters.slice(0, CHAPTERS_PREVIEW)

  const tags = manga?.tags ? manga.tags.split(',').map(t => t.trim()).filter(Boolean) : []
  const coverSrc = manga
    ? (!manga.cover_url?.startsWith('http') ? api.coverUrl(manga.id) : manga.cover_url)
    : ''

  return {
    manga,
    chapters,
    allProgress,
    queueItems,
    progressMap,
    queueMap,
    loadingManga,
    loadingChapters,
    isSyncing,
    isRemoteSource,
    total,
    downloaded,
    streamable,
    readCount,
    readPct,
    activeCount,
    pendingCount,
    resumeChapter,
    showAll,
    setShowAll,
    filterMode,
    setFilterMode,
    sortDir,
    setSortDir,
    filteredChapters,
    displayed,
    tags,
    coverSrc,
    showRemoveMenu,
    setShowRemoveMenu,
    removeMenuRef,
    pendingReadId,
    syncLog,
    openOrQueue,
    sync,
    refreshMetadata,
    removeFromQueue,
    cancelAll,
    downloadAll,
    refreshManga: fetchManga,
  }
}

export { CHAPTERS_PREVIEW }
