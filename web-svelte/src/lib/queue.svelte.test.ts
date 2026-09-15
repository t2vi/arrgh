import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'

vi.mock('./api', () => ({
  api: {
    getQueue: vi.fn(),
    removeFromQueue: vi.fn(),
    clearCompletedQueue: vi.fn(),
  },
}))

import { QueueStore } from './queue.svelte'
import { api } from './api'
import type { QueueItem } from './api'

function item(overrides: Partial<QueueItem> = {}): QueueItem {
  return {
    id: '1', chapter_id: 'c1', manga_title: 'Test', chapter_num: 1,
    status: 'pending', error: null, pages_downloaded: 0, pages_total: 0,
    created_at: '', updated_at: '',
    ...overrides,
  }
}

beforeEach(() => {
  vi.clearAllMocks()
  vi.mocked(api.getQueue).mockResolvedValue([item({ id: '1', status: 'done' }), item({ id: '2', status: 'pending' })])
  vi.mocked(api.removeFromQueue).mockResolvedValue(undefined)
  vi.mocked(api.clearCompletedQueue).mockResolvedValue(undefined)
})

afterEach(() => {
  vi.useRealTimers()
})

// QueueStore uses $effect for its poll interval and auto-clear behavior,
// which only runs inside a Svelte effect root — $effect.root() gives it one
// outside of component rendering.
function createStore() {
  let store!: QueueStore
  const cleanup = $effect.root(() => {
    store = new QueueStore()
  })
  return { store, cleanup }
}

describe('QueueStore', () => {
  it('fetches queue on construction', async () => {
    const { store, cleanup } = createStore()
    await vi.waitFor(() => expect(store.loading).toBe(false))
    expect(api.getQueue).toHaveBeenCalledTimes(1)
    expect(store.data).toHaveLength(2)
    cleanup()
  })

  it('sorts by status order (pending before done)', async () => {
    const { store, cleanup } = createStore()
    await vi.waitFor(() => expect(store.data).toBeDefined())
    expect(store.data![0].status).toBe('pending')
    expect(store.data![1].status).toBe('done')
    cleanup()
  })

  it('canClear true when completed items exist', async () => {
    const { store, cleanup } = createStore()
    await vi.waitFor(() => expect(store.loading).toBe(false))
    expect(store.canClear).toBe(true)
    cleanup()
  })

  it('canClear false when only active items', async () => {
    vi.mocked(api.getQueue).mockResolvedValue([item({ id: '1', status: 'downloading' })])
    const { store, cleanup } = createStore()
    await vi.waitFor(() => expect(store.loading).toBe(false))
    expect(store.canClear).toBe(false)
    cleanup()
  })

  it('handleRemove calls api and refetches', async () => {
    const { store, cleanup } = createStore()
    await vi.waitFor(() => expect(store.loading).toBe(false))
    await store.handleRemove('1')
    expect(api.removeFromQueue).toHaveBeenCalledWith('1')
    expect(api.getQueue).toHaveBeenCalledTimes(2)
    cleanup()
  })

  it('polls every 2s', async () => {
    vi.useFakeTimers()
    const { cleanup } = createStore()
    await vi.advanceTimersByTimeAsync(0)
    const callsBefore = vi.mocked(api.getQueue).mock.calls.length

    await vi.advanceTimersByTimeAsync(2000)
    expect(vi.mocked(api.getQueue).mock.calls.length).toBeGreaterThan(callsBefore)
    cleanup()
  })
})
