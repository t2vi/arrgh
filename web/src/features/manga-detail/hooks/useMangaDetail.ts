import { useState, useEffect, useRef, useMemo } from 'react'
import { useNavigate } from 'react-router-dom'
import { useQuery, useMutation, useQueryClient } from '@tanstack/react-query'
import { api, type QueueItem } from '@/api'
import type { Chapter, ReadProgress } from '@/types'
import { queryKeys } from '@/lib/queryKeys'
import { ROUTES } from '@/lib/routes'

export type FilterMode = 'all' | 'downloaded' | 'not_downloaded'
export type SortDir = 'desc' | 'asc'

export interface MangaDetailHandle {
  manga: ReturnType<typeof useQuery<Awaited<ReturnType<typeof api.getManga>>>>['data']
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
  // Actions
  openOrQueue: (ch: Chapter) => void
  sync: ReturnType<typeof useMutation<void, Error, void>>
  removeFromQueue: ReturnType<typeof useMutation<void, Error, string>>
  cancelAll: ReturnType<typeof useMutation<void, Error, void>>
  downloadAll: ReturnType<typeof useMutation<void, Error, void>>
}

const CHAPTERS_PREVIEW = 5

export function useMangaDetail(id: string | undefined): MangaDetailHandle {
  const navigate = useNavigate()
  const qc = useQueryClient()

  const [showAll, setShowAll] = useState(false)
  const [filterMode, setFilterMode] = useState<FilterMode>('all')
  const [sortDir, setSortDir] = useState<SortDir>('desc')
  const [showRemoveMenu, setShowRemoveMenu] = useState(false)
  const [pendingReadId, setPendingReadId] = useState<string | null>(null)
  const removeMenuRef = useRef<HTMLDivElement>(null)

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

  const { data: manga, isLoading: loadingManga } = useQuery({
    queryKey: queryKeys.manga.detail(id!),
    queryFn: () => api.getManga(id!),
    enabled: !!id,
    refetchInterval: (query) =>
      query.state.data?.sync_status === 'syncing' ? 2000 : false,
  })

  const isRemoteSource = manga?.source != null && manga.source !== 'local'
  const isSyncing = manga?.sync_status === 'syncing'

  const sync = useMutation({
    mutationFn: () => api.syncManga(id!),
    onSuccess: () => {
      qc.invalidateQueries({ queryKey: queryKeys.manga.detail(id!) })
      setTimeout(() => qc.invalidateQueries({ queryKey: queryKeys.chapters.list(id!) }), 2000)
    },
  })

  const { data: chapters = [], isLoading: loadingChapters } = useQuery<Chapter[]>({
    queryKey: queryKeys.chapters.list(id!),
    queryFn: () => api.listChapters(id!),
    refetchInterval: isSyncing ? 3000 : false,
    enabled: !!manga,
  })

  const { data: allProgress = [] } = useQuery({
    queryKey: queryKeys.progress.manga(id!),
    queryFn: () => api.getMangaProgress(id!),
    enabled: chapters.length > 0,
  })

  const progressMap = new Map<string, ReadProgress>(allProgress.map((p) => [p.chapter_id, p]))

  const { data: queueItems = [] } = useQuery({
    queryKey: queryKeys.queue.manga(id!),
    queryFn: () => api.getMangaQueue(id!),
    refetchInterval: 2000,
    enabled: !!id,
  })

  const queueMap = new Map<string, QueueItem>(queueItems.map((q) => [q.chapter_id, q]))

  const prevQueueRef = useRef<typeof queueItems>([])
  useEffect(() => {
    const prev = prevQueueRef.current
    const justDone = queueItems.some(
      (q) => q.status === 'done' && prev.find((p) => p.id === q.id)?.status !== 'done',
    )
    if (justDone) qc.invalidateQueries({ queryKey: queryKeys.chapters.list(id!) })
    prevQueueRef.current = queueItems
  }, [queueItems, id, qc])

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
    if (!ch.source_id) return
    const qi = queueMap.get(ch.id)
    if (!qi || qi.status === 'error' || qi.status === 'cancelled') {
      api.downloadChapter(ch.id)
        .then(() => qc.invalidateQueries({ queryKey: queryKeys.queue.manga(id!) }))
        .catch(() => {})
    }
    setPendingReadId(ch.id)
  }

  const removeFromQueue = useMutation({ mutationFn: api.removeFromQueue })

  const cancelAll = useMutation({
    mutationFn: async () => {
      for (const q of queueItems.filter((q) => q.status === 'pending' || q.status === 'downloading')) {
        await api.removeFromQueue(q.id).catch(() => {})
      }
    },
    onSuccess: () => qc.invalidateQueries({ queryKey: queryKeys.queue.manga(id!) }),
  })

  const downloadAll = useMutation({
    mutationFn: async () => {
      for (const ch of chapters.filter((c) => !c.downloaded && c.source_id)) {
        await api.downloadChapter(ch.id).catch(() => {})
      }
    },
    onSuccess: () => qc.invalidateQueries({ queryKey: queryKeys.queue.all() }),
  })

  const total = chapters.length
  const downloaded = chapters.filter((c) => c.downloaded).length
  const streamable = chapters.filter((c) => !c.downloaded && c.source_id).length
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

  const tags = manga?.tags ? manga.tags.split(', ').filter(Boolean) : []
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
    openOrQueue,
    sync,
    removeFromQueue,
    cancelAll,
    downloadAll,
  }
}

export { CHAPTERS_PREVIEW }
