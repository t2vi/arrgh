import { api, type QueueItem } from './api'
import type { Chapter, ReadProgress, SyncLogEntry, Title } from './types'
import { router } from './router.svelte'
import { ROUTES } from './routes'

export type FilterMode = 'all' | 'downloaded' | 'not_downloaded'
export type SortDir = 'desc' | 'asc'

export const CHAPTERS_PREVIEW = 5

export class MangaDetailStore {
  #id = $state<string | undefined>(undefined)

  manga = $state<Title | undefined>(undefined)
  loadingManga = $state(true)
  chapters = $state<Chapter[]>([])
  loadingChapters = $state(true)
  allProgress = $state<ReadProgress[]>([])
  queueItems = $state<QueueItem[]>([])
  syncLog = $state<SyncLogEntry[]>([])

  showAll = $state(false)
  filterMode = $state<FilterMode>('all')
  sortDir = $state<SortDir>('desc')
  showRemoveMenu = $state(false)
  pendingReadId = $state<string | null>(null)

  syncPending = $state(false)
  refreshMetaPending = $state(false)
  removeFromQueuePending = $state(false)
  cancelAllPending = $state(false)
  downloadAllPending = $state(false)

  #prevQueue: QueueItem[] = []

  get progressMap() {
    return new Map(this.allProgress.map((p) => [p.chapter_id, p]))
  }
  get queueMap() {
    return new Map(this.queueItems.map((q) => [q.chapter_id, q]))
  }
  get isRemoteSource() {
    return this.manga != null && !this.manga.is_local
  }
  get isSyncing() {
    return this.manga?.sync_status === 'syncing'
  }
  get total() {
    return this.chapters.length
  }
  get downloaded() {
    return this.chapters.filter((c) => c.downloaded).length
  }
  get streamable() {
    return this.chapters.filter((c) => !c.downloaded && c.has_sources).length
  }
  get readCount() {
    return this.allProgress.filter((p) => p.completed).length
  }
  get readPct() {
    return this.total > 0 ? Math.round((this.readCount / this.total) * 100) : 0
  }
  get activeCount() {
    return this.queueItems.filter((q) => q.status === 'downloading').length
  }
  get pendingCount() {
    return this.queueItems.filter((q) => q.status === 'pending').length
  }
  get resumeChapter() {
    const pm = this.progressMap
    const inProgress = this.chapters.find((c) => {
      const p = pm.get(c.id)
      return p && !p.completed
    })
    if (inProgress) return inProgress
    return [...this.chapters].sort((a, b) => a.number - b.number).find((c) => !pm.get(c.id)?.completed)
  }
  get filteredChapters() {
    let result = [...this.chapters]
    if (this.filterMode === 'downloaded') result = result.filter((c) => c.downloaded)
    else if (this.filterMode === 'not_downloaded') result = result.filter((c) => !c.downloaded)
    result.sort((a, b) => (this.sortDir === 'asc' ? a.number - b.number : b.number - a.number))
    return result
  }
  get displayed() {
    return this.showAll ? this.filteredChapters : this.filteredChapters.slice(0, CHAPTERS_PREVIEW)
  }
  get tags() {
    return this.manga?.tags ? this.manga.tags.split(',').map((t) => t.trim()).filter(Boolean) : []
  }
  get coverSrc() {
    if (!this.manga) return ''
    return !this.manga.cover_url?.startsWith('http') ? api.coverUrl(this.manga.id) : this.manga.cover_url
  }

  constructor() {
    $effect(() => {
      if (!this.#id) return
      this.loadingManga = true
      this.fetchManga()
    })

    $effect(() => {
      if (!this.manga) return
      this.fetchChapters()
      this.fetchProgress()
    })

    // Poll manga every 2s while syncing.
    $effect(() => {
      if (this.manga?.sync_status !== 'syncing') return
      const t = setInterval(() => this.fetchManga(), 2000)
      return () => clearInterval(t)
    })

    // Poll chapters every 3s while syncing.
    $effect(() => {
      if (this.manga?.sync_status !== 'syncing') return
      const t = setInterval(() => this.fetchChapters(), 3000)
      return () => clearInterval(t)
    })

    $effect(() => {
      if (!this.#id) return
      this.fetchQueue()
    })

    $effect(() => {
      if (!this.#id) return
      this.fetchSyncLog()
    })

    $effect(() => {
      if (this.manga?.sync_status !== 'syncing') return
      const t = setInterval(() => this.fetchSyncLog(), 2000)
      return () => clearInterval(t)
    })

    // Poll every 2s only while there are active queue items; stops automatically when idle.
    $effect(() => {
      const hasActive = this.queueItems.some((q) => q.status === 'pending' || q.status === 'downloading')
      if (!hasActive) return
      const t = setInterval(() => this.fetchQueue(), 2000)
      return () => clearInterval(t)
    })

    // Re-fetch chapters once a queued download flips to "done".
    $effect(() => {
      const items = this.queueItems
      const prev = this.#prevQueue
      const justDone = items.some((q) => q.status === 'done' && prev.find((p) => p.id === q.id)?.status !== 'done')
      if (justDone) this.fetchChapters()
      this.#prevQueue = items
    })

    // Once a chapter queued via openOrQueue finishes, jump into the reader.
    $effect(() => {
      if (!this.pendingReadId) return
      const qi = this.queueMap.get(this.pendingReadId)
      if (qi?.status === 'error') {
        this.pendingReadId = null
        return
      }
      const ch = this.chapters.find((c) => c.id === this.pendingReadId)
      if (ch?.downloaded || qi?.status === 'done') {
        const readId = this.pendingReadId
        this.pendingReadId = null
        router.navigate(ROUTES.reader(readId))
      }
    })
  }

  load(id: string) {
    this.#id = id
  }

  fetchManga = () => {
    if (!this.#id) return
    api
      .getTitle(this.#id)
      .then((m) => (this.manga = m))
      .catch(() => {})
      .finally(() => (this.loadingManga = false))
  }

  fetchChapters = () => {
    if (!this.#id) return
    api
      .listChapters(this.#id)
      .then((c) => (this.chapters = c))
      .catch(() => {})
      .finally(() => (this.loadingChapters = false))
  }

  fetchProgress = () => {
    if (!this.#id) return
    api.getTitleProgress(this.#id).then((p) => (this.allProgress = p)).catch(() => {})
  }

  fetchQueue = () => {
    if (!this.#id) return
    api.getTitleQueue(this.#id).then((q) => (this.queueItems = q)).catch(() => {})
  }

  fetchSyncLog = () => {
    if (!this.#id) return
    api.getSyncLog(this.#id).then((l) => (this.syncLog = l)).catch(() => {})
  }

  openOrQueue(ch: Chapter) {
    if (ch.downloaded) {
      router.navigate(ROUTES.reader(ch.id))
      return
    }
    if (!ch.has_sources) return
    const qi = this.queueMap.get(ch.id)
    if (!qi || qi.status === 'error' || qi.status === 'cancelled') {
      api.downloadChapter(ch.id).then(() => this.fetchQueue()).catch(() => {})
    }
    this.pendingReadId = ch.id
  }

  sync() {
    if (!this.#id) return
    this.syncPending = true
    api
      .syncTitle(this.#id)
      .then(() => {
        this.fetchManga()
        setTimeout(() => this.fetchChapters(), 2000)
      })
      .catch(() => {})
      .finally(() => (this.syncPending = false))
  }

  refreshMetadata() {
    if (!this.#id) return
    this.refreshMetaPending = true
    api
      .refreshMetadata(this.#id)
      .then(() => {
        this.fetchManga()
        setTimeout(() => this.fetchChapters(), 3000)
      })
      .catch(() => {})
      .finally(() => (this.refreshMetaPending = false))
  }

  removeFromQueue(itemId: string) {
    this.removeFromQueuePending = true
    api
      .removeFromQueue(itemId)
      .catch(() => {})
      .finally(() => (this.removeFromQueuePending = false))
  }

  cancelAll() {
    this.cancelAllPending = true
    const toCancel = this.queueItems.filter((q) => q.status === 'pending' || q.status === 'downloading')
    Promise.all(toCancel.map((q) => api.removeFromQueue(q.id).catch(() => {})))
      .then(() => this.fetchQueue())
      .finally(() => (this.cancelAllPending = false))
  }

  downloadAll() {
    this.downloadAllPending = true
    const toDownload = this.chapters.filter((c) => !c.downloaded && c.has_sources)
    Promise.all(toDownload.map((c) => api.downloadChapter(c.id).catch(() => {})))
      .then(() => this.fetchQueue())
      .finally(() => (this.downloadAllPending = false))
  }

  refreshManga() {
    this.fetchManga()
  }
}
