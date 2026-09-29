import { beforeEach, describe, expect, it, vi } from 'vitest'

vi.mock('./api', () => ({
  api: { searchMangaStream: vi.fn(), addTitle: vi.fn() },
}))

import { DiscoverStore } from './discover.svelte'
import { api, type StreamEvent } from './api'
import { router } from './router.svelte'
import { ROUTES } from './routes'

const mockResult = {
  mangaupdates_id: 'mu-001',
  title: 'Test',
  description: null,
  cover_url: null,
  status: 'ongoing',
  author: null,
  year: null,
  tags: null,
  content_type: 'manga',
  in_library: false,
  library_id: null,
  source: 'mangaupdates',
  is_explicit: false,
}

const anilistResult = { ...mockResult, source: 'anilist', mangaupdates_id: 'al-999', content_type: 'manhwa' }

beforeEach(() => {
  vi.clearAllMocks()
  vi.spyOn(router, 'navigate').mockImplementation(() => {})
  streamWith([])
})

const SOURCES = [
  { key: 'mangaupdates', label: 'MangaUpdates' },
  { key: 'anilist', label: 'AniList' },
]

/** Mock the stream: replay `events`, then resolve. */
function streamWith(events: StreamEvent[]) {
  vi.mocked(api.searchMangaStream).mockImplementation(async (_q, onEvent) => {
    for (const e of events) onEvent(e)
  })
}

/** A full successful stream whose final results are `results`. */
function resultsStream(results: unknown[]): StreamEvent[] {
  return [
    { type: 'sources', sources: SOURCES },
    { type: 'source', key: 'mangaupdates', status: results.length ? 'found' : 'empty', count: results.length, ms: 5, results: results as never },
    { type: 'done', ok: true },
  ]
}

function createStore() {
  let store!: DiscoverStore
  const cleanup = $effect.root(() => {
    store = new DiscoverStore()
  })
  return { store, cleanup }
}

describe('DiscoverStore', () => {
  it('submit triggers searchMangaStream with current query', async () => {
    const { store, cleanup } = createStore()
    store.query = 'naruto'
    store.submit()
    await vi.waitFor(() => expect(api.searchMangaStream).toHaveBeenCalledWith('naruto', expect.any(Function), expect.any(AbortSignal)))
    cleanup()
  })

  it('submit does nothing when query is blank', () => {
    const { store, cleanup } = createStore()
    store.submit()
    expect(api.searchMangaStream).not.toHaveBeenCalled()
    cleanup()
  })

  it('navigates to library after handleAdd succeeds', async () => {
    vi.mocked(api.addTitle).mockResolvedValue({ id: 'manga-1', title: 'Test' } as never)
    const { store, cleanup } = createStore()
    await store.handleAdd(mockResult as never)
    expect(router.navigate).toHaveBeenCalledWith(ROUTES.library)
    cleanup()
  })

  it('sets searchError on 502 with generic discovery message', async () => {
    vi.mocked(api.searchMangaStream).mockRejectedValue(new Error('502 Bad Gateway'))
    const { store, cleanup } = createStore()
    store.query = 'test'
    store.submit()
    await vi.waitFor(() => expect(store.searchError).toBeTruthy())
    expect(store.searchError).not.toMatch(/^MangaUpdates search failed/)
    cleanup()
  })

  it('tracks added manga by mangaupdates_id', async () => {
    vi.mocked(api.addTitle).mockResolvedValue({ id: 'manga-99', title: 'Test' } as never)
    const { store, cleanup } = createStore()
    await store.handleAdd(mockResult as never)
    expect(store.added.get('mu-001')).toBe('manga-99')
    cleanup()
  })

  it('tracks added manga for non-MU sources', async () => {
    vi.mocked(api.addTitle).mockResolvedValue({ id: 'manga-anilist', title: 'Test Manhwa' } as never)
    const { store, cleanup } = createStore()
    await store.handleAdd(anilistResult as never)
    expect(store.added.get('al-999')).toBe('manga-anilist')
    cleanup()
  })

  it('addingId is set while add is in flight, then cleared', async () => {
    let resolveAdd!: (v: unknown) => void
    vi.mocked(api.addTitle).mockImplementation(() => new Promise((res) => (resolveAdd = res as (v: unknown) => void)))
    const { store, cleanup } = createStore()
    const p = store.handleAdd(mockResult as never)
    expect(store.addingId).toBe('mu-001')
    resolveAdd({ id: 'manga-1' })
    await p
    expect(store.addingId).toBeNull()
    cleanup()
  })

  it('sets addError when addTitle throws', async () => {
    vi.mocked(api.addTitle).mockRejectedValue(new Error('Not found'))
    const { store, cleanup } = createStore()
    await store.handleAdd(mockResult as never)
    expect(store.addError).toBe('Not found')
    cleanup()
  })

  it('availableTypes derived from all result content_type values', async () => {
    const mixed = [
      { ...mockResult, content_type: 'manga' },
      { ...anilistResult, content_type: 'manhwa' },
      { ...mockResult, mangaupdates_id: 'nu-1', source: 'novelupdates', content_type: 'novel' },
    ]
    streamWith(resultsStream(mixed))
    const { store, cleanup } = createStore()
    store.query = 'test'
    store.submit()
    await vi.waitFor(() => expect(store.availableTypes).toEqual(new Set(['manga', 'manhwa', 'novel'])))
    cleanup()
  })

  it('filteredData returns only matching content_type when filter set', async () => {
    const mixed = [
      { ...mockResult, content_type: 'manga' },
      { ...anilistResult, content_type: 'manhwa' },
    ]
    streamWith(resultsStream(mixed))
    const { store, cleanup } = createStore()
    store.query = 'test'
    store.submit()
    await vi.waitFor(() => expect(store.data).toHaveLength(2))
    store.setContentTypeFilter('manhwa')
    expect(store.filteredData).toHaveLength(1)
    expect(store.filteredData![0].content_type).toBe('manhwa')
    cleanup()
  })

  it('setContentTypeFilter toggles off when called with current value', async () => {
    streamWith(resultsStream([mockResult]))
    const { store, cleanup } = createStore()
    store.query = 'test'
    store.submit()
    await vi.waitFor(() => expect(store.data).toBeDefined())
    store.setContentTypeFilter('manga')
    expect(store.contentTypeFilter).toBe('manga')
    store.setContentTypeFilter('manga')
    expect(store.contentTypeFilter).toBeUndefined()
    cleanup()
  })

  it('resets contentTypeFilter when a new search is submitted', async () => {
    streamWith(resultsStream([mockResult]))
    const { store, cleanup } = createStore()
    store.query = 'first'
    store.submit()
    await vi.waitFor(() => expect(store.data).toBeDefined())
    store.setContentTypeFilter('manga')
    store.query = 'second'
    store.submit()
    await vi.waitFor(() => expect(store.contentTypeFilter).toBeUndefined())
    cleanup()
  })

  describe('streaming (spec 021)', () => {
    // spec: 021/FR-002, 021/FR-007, 021/FR-011
    it('sources event → every source is searching', async () => {
      vi.mocked(api.searchMangaStream).mockImplementation(async (_q, onEvent) => {
        onEvent({ type: 'sources', sources: SOURCES })
        await new Promise(() => {}) // never finishes
      })
      const { store, cleanup } = createStore()
      store.query = 'x'
      store.submit()
      await vi.waitFor(() => expect(store.sources).toEqual(SOURCES))
      expect([...store.sourceState.values()].every((s) => s.status === 'searching')).toBe(true)
      expect(store.isFetching).toBe(true)
      cleanup()
    })

    // spec: 021/FR-003, 021/FR-007, 021/FR-008
    it('source events update per-source progress while fetching, but data stays held back until done', async () => {
      let emit!: (e: StreamEvent) => void
      vi.mocked(api.searchMangaStream).mockImplementation((_q, onEvent) => {
        emit = onEvent
        return new Promise(() => {})
      })
      const { store, cleanup } = createStore()
      store.query = 'x'
      store.submit()
      await vi.waitFor(() => expect(emit).toBeDefined())
      emit({ type: 'sources', sources: SOURCES })
      emit({ type: 'source', key: 'anilist', status: 'found', count: 1, ms: 5, results: [anilistResult] as never })
      expect(store.data).toBeUndefined()
      expect(store.isFetching).toBe(true)
      expect(store.sourceState.get('anilist')).toEqual({ status: 'found', count: 1 })
      expect(store.sourceState.get('mangaupdates')).toEqual({ status: 'searching' })

      emit({ type: 'source', key: 'mangaupdates', status: 'found', count: 1, ms: 9, results: [mockResult, anilistResult] as never })
      expect(store.data).toBeUndefined()
      emit({ type: 'done', ok: true })
      expect(store.data).toHaveLength(2)
      expect(store.isFetching).toBe(false)
      expect(store.searchError).toBeNull()
      cleanup()
    })

    it('done{ok:false} → discovery-failed error', async () => {
      streamWith([
        { type: 'sources', sources: SOURCES },
        { type: 'source', key: 'mangaupdates', status: 'error', count: 0, ms: 5, results: [] },
        { type: 'source', key: 'anilist', status: 'timeout', count: 0, ms: 9, results: [] },
        { type: 'done', ok: false },
      ])
      const { store, cleanup } = createStore()
      store.query = 'x'
      store.submit()
      await vi.waitFor(() => expect(store.searchError).toBe('Discovery failed. Check your connection or server status.'))
      expect(store.isFetching).toBe(false)
      cleanup()
    })

    // spec: 021/FR-009
    it('a new submit aborts the previous stream and ignores its late events', async () => {
      const calls: { emit: (e: StreamEvent) => void; signal: AbortSignal }[] = []
      vi.mocked(api.searchMangaStream).mockImplementation((_q, onEvent, signal) => {
        calls.push({ emit: onEvent, signal })
        return new Promise(() => {})
      })
      const { store, cleanup } = createStore()
      store.query = 'first'
      store.submit()
      await vi.waitFor(() => expect(calls).toHaveLength(1))
      store.query = 'second'
      store.submit()
      await vi.waitFor(() => expect(calls).toHaveLength(2))
      expect(calls[0].signal.aborted).toBe(true)

      calls[0].emit({ type: 'source', key: 'mangaupdates', status: 'found', count: 1, ms: 1, results: [mockResult] as never })
      calls[0].emit({ type: 'done', ok: true })
      expect(store.data).toBeUndefined()
      calls[1].emit({ type: 'source', key: 'anilist', status: 'found', count: 1, ms: 1, results: [anilistResult] as never })
      calls[1].emit({ type: 'done', ok: true })
      expect(store.data).toEqual([anilistResult])
      cleanup()
    })

    // spec: 021/FR-009
    it('cleanup (leaving the page) aborts the stream', async () => {
      let signal!: AbortSignal
      vi.mocked(api.searchMangaStream).mockImplementation((_q, _onEvent, s) => {
        signal = s
        return new Promise(() => {})
      })
      const { store, cleanup } = createStore()
      store.query = 'x'
      store.submit()
      await vi.waitFor(() => expect(signal).toBeDefined())
      cleanup()
      expect(signal.aborted).toBe(true)
    })
  })
})
