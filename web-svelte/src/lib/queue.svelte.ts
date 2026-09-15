import { api, type QueueItem } from './api'

const STATUS_ORDER: Record<string, number> = {
  downloading: 0, pending: 1, done: 2, error: 3, cancelled: 4,
}

function isActive(item: QueueItem) {
  return item.status === 'pending' || item.status === 'downloading'
}

export class QueueStore {
  raw = $state<QueueItem[] | undefined>(undefined)
  loading = $state(true)
  clearingCompleted = $state(false)
  #hadActive = false

  get data() {
    return this.raw?.slice().sort((a, b) => (STATUS_ORDER[a.status] ?? 9) - (STATUS_ORDER[b.status] ?? 9))
  }

  get canClear() {
    const d = this.data
    const hasCompletedItems = d?.some((i) => !isActive(i) && i.status !== 'error') ?? false
    const hasErrorItems = d?.some((i) => i.status === 'error') ?? false
    return hasCompletedItems || hasErrorItems
  }

  constructor() {
    this.fetchQueue()

    $effect(() => {
      const id = setInterval(() => this.fetchQueue(), 2000)
      return () => clearInterval(id)
    })

    $effect(() => {
      const d = this.data
      if (!d) return
      const hasActive = d.some(isActive)
      const hasFinished = d.some((i) => i.status === 'done' || i.status === 'cancelled')
      if (this.#hadActive && !hasActive && hasFinished) {
        this.handleClearCompleted()
      }
      this.#hadActive = hasActive
    })
  }

  fetchQueue = () => {
    api.getQueue()
      .then((d) => (this.raw = d))
      .catch(() => {})
      .finally(() => (this.loading = false))
  }

  handleRemove = async (itemId: string) => {
    await api.removeFromQueue(itemId).catch(() => {})
    this.fetchQueue()
  }

  handleClearCompleted = async () => {
    this.clearingCompleted = true
    try {
      await api.clearCompletedQueue()
      this.fetchQueue()
    } catch {
      // ignore
    } finally {
      this.clearingCompleted = false
    }
  }
}
