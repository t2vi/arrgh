import { beforeEach, describe, expect, it, vi } from 'vitest'

vi.mock('./api', () => ({
  api: {
    listTitles: vi.fn(),
    getTrendingManga: vi.fn(),
    getTrendingManhwa: vi.fn(),
    getTrendingManhua: vi.fn(),
    getTrendingAdultManhwa: vi.fn(),
    getNewReleases: vi.fn(),
    getContinueReading: vi.fn(),
  },
  getAllowExplicit: vi.fn(() => false),
}))

import { HomeStore } from './home.svelte'
import { api, getAllowExplicit } from './api'

const emptyPage = { items: [], total: 0, page: 1, limit: 20 }

function makeTrendingResult(overrides: object = {}) {
  return {
    mangaupdates_id: 't1',
    source: 'mangadex',
    title: 'Berserk',
    description: null,
    cover_url: null,
    status: 'ongoing',
    author: null,
    year: null,
    tags: null,
    content_type: 'manga',
    in_library: false,
    library_id: null,
    is_explicit: false,
    ...overrides,
  }
}

beforeEach(() => {
  vi.clearAllMocks()
  vi.mocked(getAllowExplicit).mockReturnValue(false)
  vi.mocked(api.listTitles).mockResolvedValue(emptyPage as never)
  vi.mocked(api.getTrendingManga).mockResolvedValue([])
  vi.mocked(api.getTrendingManhwa).mockResolvedValue([])
  vi.mocked(api.getTrendingManhua).mockResolvedValue([])
  vi.mocked(api.getTrendingAdultManhwa).mockResolvedValue([])
  vi.mocked(api.getNewReleases).mockResolvedValue([])
  vi.mocked(api.getContinueReading).mockResolvedValue([])
})

function createStore() {
  let store!: HomeStore
  const cleanup = $effect.root(() => {
    store = new HomeStore()
  })
  return { store, cleanup }
}

describe('HomeStore', () => {
  it('loads trending manga on creation', async () => {
    vi.mocked(api.getTrendingManga).mockResolvedValue([
      makeTrendingResult({ cover_url: 'https://cdn.example.com/cover.jpg', in_library: false }),
    ] as never)
    const { store, cleanup } = createStore()
    await vi.waitFor(() => expect(store.trendingMangaLoading).toBe(false))
    expect(store.trendingManga).toHaveLength(1)
    expect(store.trendingManga[0].title).toBe('Berserk')
    cleanup()
  })

  it('filters in-library titles from trending', async () => {
    vi.mocked(api.getTrendingManga).mockResolvedValue([
      makeTrendingResult({ cover_url: 'https://cdn.example.com/1.jpg', in_library: false }),
      makeTrendingResult({
        mangaupdates_id: 't2',
        title: 'One Piece',
        cover_url: 'https://cdn.example.com/2.jpg',
        in_library: true,
      }),
    ] as never)
    const { store, cleanup } = createStore()
    await vi.waitFor(() => expect(store.trendingMangaLoading).toBe(false))
    expect(store.trendingManga.every((r) => !r.in_library)).toBe(true)
    expect(store.trendingManga.some((r) => r.title === 'One Piece')).toBe(false)
    cleanup()
  })

  it('trendingMangaLoading is true initially and false after fetch', async () => {
    const { store, cleanup } = createStore()
    expect(store.trendingMangaLoading).toBe(true)
    await vi.waitFor(() => expect(store.trendingMangaLoading).toBe(false))
    cleanup()
  })

  it('skips the adult-manhwa lane when explicit content is not allowed', async () => {
    const { store, cleanup } = createStore()
    await vi.waitFor(() => expect(store.trendingAdultManhwaLoading).toBe(false))
    expect(api.getTrendingAdultManhwa).not.toHaveBeenCalled()
    cleanup()
  })

  it('fetches the adult-manhwa lane when explicit content is allowed', async () => {
    vi.mocked(getAllowExplicit).mockReturnValue(true)
    vi.mocked(api.getTrendingAdultManhwa).mockResolvedValue([makeTrendingResult()] as never)
    const { store, cleanup } = createStore()
    await vi.waitFor(() => expect(store.trendingAdultManhwaLoading).toBe(false))
    expect(api.getTrendingAdultManhwa).toHaveBeenCalled()
    expect(store.trendingAdultManhwa).toHaveLength(1)
    cleanup()
  })

  it('isLoading is true initially and false after the library list resolves', async () => {
    const { store, cleanup } = createStore()
    expect(store.isLoading).toBe(true)
    await vi.waitFor(() => expect(store.isLoading).toBe(false))
    cleanup()
  })

  it('coverManga prefers the first continue-reading item over the first library item', async () => {
    vi.mocked(api.listTitles).mockResolvedValue({
      items: [{ id: 'm1', cover_url: 'lib.jpg' }],
      total: 1,
      page: 1,
      limit: 20,
    } as never)
    vi.mocked(api.getContinueReading).mockResolvedValue([
      { title_id: 'c1', cover_url: 'continue.jpg', chapter_id: 'ch1', chapter_number: 1, chapters_read: 1, total_chapters: 2, manga_title: 'X' },
    ] as never)
    const { store, cleanup } = createStore()
    await vi.waitFor(() => expect(store.coverManga?.id).toBe('c1'))
    cleanup()
  })

  it('totalRead sums chapters_read across items', async () => {
    vi.mocked(api.listTitles).mockResolvedValue({
      items: [{ id: 'm1', chapters_read: 3 }, { id: 'm2', chapters_read: 5 }],
      total: 2,
      page: 1,
      limit: 20,
    } as never)
    const { store, cleanup } = createStore()
    await vi.waitFor(() => expect(store.isLoading).toBe(false))
    expect(store.totalRead).toBe(8)
    cleanup()
  })
})
