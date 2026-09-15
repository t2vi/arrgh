import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'

vi.mock('./api', () => ({
  api: {
    listTitles: vi.fn(),
    removeTitle: vi.fn(),
    getSyncLog: vi.fn().mockResolvedValue([]),
  },
}))

import { LibraryStore } from './library.svelte'
import { api } from './api'

const mockPage = {
  items: [{ id: 'm1', title: 'Naruto', sync_status: 'ready' }],
  total: 1,
  page: 1,
  limit: 20,
}

beforeEach(() => {
  vi.clearAllMocks()
  vi.mocked(api.listTitles).mockResolvedValue(mockPage as never)
  vi.mocked(api.removeTitle).mockResolvedValue(undefined as never)
})

afterEach(() => {
  vi.useRealTimers()
})

// LibraryStore uses $effect for its fetch-on-change and polling behavior,
// which only runs inside a Svelte effect root — $effect.root() gives it one
// outside of component rendering.
function createStore() {
  let store!: LibraryStore
  const cleanup = $effect.root(() => {
    store = new LibraryStore()
  })
  return { store, cleanup }
}

describe('LibraryStore', () => {
  it('fetches on creation and sets data', async () => {
    const { store, cleanup } = createStore()
    await vi.waitFor(() => expect(store.loading).toBe(false))
    expect(api.listTitles).toHaveBeenCalledTimes(1)
    expect(store.data?.items).toHaveLength(1)
    cleanup()
  })

  it('computes totalPages correctly', async () => {
    vi.mocked(api.listTitles).mockResolvedValue({ items: [], total: 45, page: 1, limit: 20 } as never)
    const { store, cleanup } = createStore()
    await vi.waitFor(() => expect(store.loading).toBe(false))
    expect(store.totalPages).toBe(3)
    cleanup()
  })

  it('handleRemove calls removeTitle then refetches', async () => {
    const { store, cleanup } = createStore()
    await vi.waitFor(() => expect(store.loading).toBe(false))
    await store.handleRemove('m1', false)
    expect(api.removeTitle).toHaveBeenCalledWith('m1', false)
    expect(api.listTitles).toHaveBeenCalledTimes(2)
    cleanup()
  })

  it('sets removingId during removal then clears it', async () => {
    let resolve!: () => void
    vi.mocked(api.removeTitle).mockReturnValue(new Promise<void>((r) => (resolve = r)) as never)
    const { store, cleanup } = createStore()
    await vi.waitFor(() => expect(store.loading).toBe(false))

    const p = store.handleRemove('m1', false)
    expect(store.removingId).toBe('m1')
    resolve()
    await p
    expect(store.removingId).toBeNull()
    cleanup()
  })

  it('sort defaults to "recent"', () => {
    const { store, cleanup } = createStore()
    expect(store.sort).toBe('recent')
    cleanup()
  })

  it('setSort resets page to 1 and refetches with new sort', async () => {
    const { store, cleanup } = createStore()
    await vi.waitFor(() => expect(store.loading).toBe(false))
    vi.clearAllMocks()
    vi.mocked(api.listTitles).mockResolvedValue(mockPage as never)

    store.setSort('title_asc')
    await vi.waitFor(() => expect(api.listTitles).toHaveBeenCalled())
    expect(api.listTitles).toHaveBeenCalledWith(1, undefined, 'title_asc', undefined, undefined)
    cleanup()
  })

  it('toggleContentType adds then removes value', async () => {
    const { store, cleanup } = createStore()
    await vi.waitFor(() => expect(store.loading).toBe(false))

    store.toggleContentType('manga')
    expect(store.contentTypes).toEqual(['manga'])

    store.toggleContentType('manga')
    expect(store.contentTypes).toEqual([])
    cleanup()
  })

  it('toggleContentType resets page to 1', async () => {
    const { store, cleanup } = createStore()
    await vi.waitFor(() => expect(store.loading).toBe(false))
    store.setPage(3)

    store.toggleContentType('manhwa')
    expect(store.page).toBe(1)
    cleanup()
  })

  it('toggleStatus adds then removes value', async () => {
    const { store, cleanup } = createStore()
    await vi.waitFor(() => expect(store.loading).toBe(false))

    store.toggleStatus('ongoing')
    expect(store.statuses).toEqual(['ongoing'])

    store.toggleStatus('ongoing')
    expect(store.statuses).toEqual([])
    cleanup()
  })

  it('hasFilters is false with no filters, true when any active', async () => {
    const { store, cleanup } = createStore()
    await vi.waitFor(() => expect(store.loading).toBe(false))
    expect(store.hasFilters).toBe(false)

    store.toggleContentType('manga')
    expect(store.hasFilters).toBe(true)
    cleanup()
  })

  it('clearFilters resets contentTypes and statuses and resets page', async () => {
    const { store, cleanup } = createStore()
    await vi.waitFor(() => expect(store.loading).toBe(false))

    store.toggleContentType('manga')
    store.toggleStatus('ongoing')
    expect(store.hasFilters).toBe(true)

    store.clearFilters()
    expect(store.contentTypes).toEqual([])
    expect(store.statuses).toEqual([])
    expect(store.hasFilters).toBe(false)
    expect(store.page).toBe(1)
    cleanup()
  })

  it('fetches with contentType and status params when filters active', async () => {
    const { store, cleanup } = createStore()
    await vi.waitFor(() => expect(store.loading).toBe(false))
    vi.clearAllMocks()
    vi.mocked(api.listTitles).mockResolvedValue(mockPage as never)

    store.toggleContentType('manga')
    store.toggleStatus('ongoing')
    await vi.waitFor(() => expect(api.listTitles).toHaveBeenCalled())
    expect(api.listTitles).toHaveBeenCalledWith(1, undefined, 'recent', ['manga'], ['ongoing'])
    cleanup()
  })

  it('showFilters defaults false, and is directly settable', () => {
    const { store, cleanup } = createStore()
    expect(store.showFilters).toBe(false)
    store.showFilters = true
    expect(store.showFilters).toBe(true)
    cleanup()
  })

  it('polls every 2s when a manga is syncing', async () => {
    vi.useFakeTimers()
    const syncingPage = {
      items: [{ id: 'm1', title: 'X', sync_status: 'syncing' }],
      total: 1,
      page: 1,
      limit: 20,
    }
    vi.mocked(api.listTitles).mockResolvedValue(syncingPage as never)

    const { cleanup } = createStore()
    await vi.advanceTimersByTimeAsync(0)
    const callsBefore = vi.mocked(api.listTitles).mock.calls.length

    await vi.advanceTimersByTimeAsync(2000)
    expect(vi.mocked(api.listTitles).mock.calls.length).toBeGreaterThan(callsBefore)
    cleanup()
  })
})
