import { beforeEach, describe, expect, it, vi } from 'vitest'

vi.mock('./api', () => ({
  api: { searchManga: vi.fn(), addTitle: vi.fn() },
}))

import { DiscoverStore } from './discover.svelte'
import { api } from './api'
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
  vi.mocked(api.searchManga).mockResolvedValue([])
})

function createStore() {
  let store!: DiscoverStore
  const cleanup = $effect.root(() => {
    store = new DiscoverStore()
  })
  return { store, cleanup }
}

describe('DiscoverStore', () => {
  it('submit triggers searchManga with current query', async () => {
    const { store, cleanup } = createStore()
    store.query = 'naruto'
    store.submit()
    await vi.waitFor(() => expect(api.searchManga).toHaveBeenCalledWith('naruto'))
    cleanup()
  })

  it('submit does nothing when query is blank', () => {
    const { store, cleanup } = createStore()
    store.submit()
    expect(api.searchManga).not.toHaveBeenCalled()
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
    vi.mocked(api.searchManga).mockRejectedValue(new Error('502 Bad Gateway'))
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
    vi.mocked(api.searchManga).mockResolvedValue(mixed as never)
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
    vi.mocked(api.searchManga).mockResolvedValue(mixed as never)
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
    vi.mocked(api.searchManga).mockResolvedValue([mockResult] as never)
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
    vi.mocked(api.searchManga).mockResolvedValue([mockResult] as never)
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
})
