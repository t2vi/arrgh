import { renderHook, act, waitFor } from '@testing-library/react'
import { vi, describe, it, expect, beforeEach, afterEach } from 'vitest'

vi.mock('@/api', () => ({
  getAllowExplicit: vi.fn().mockReturnValue(false),
  api: {
    listTitles: vi.fn(),
    getTrending: vi.fn(),
    getTrendingManga: vi.fn(),
    getTrendingManhwa: vi.fn(),
    getTrendingManhua: vi.fn(),
    getTrendingAdultManhwa: vi.fn(),
    getNewReleases: vi.fn(),
    getContinueReading: vi.fn(),
  },
}))

import { useHome } from './useHome'
import { api } from '@/api'
beforeEach(async () => {
  await allure.epic('Home')
  await allure.feature('Home')
})

const emptyPage = { items: [], total: 0, page: 1, limit: 20 }

function makeTrendingResult(overrides: object = {}) {
  return {
    id: 't1', source: 'mangadex', source_name: 'MangaDex', title: 'Berserk',
    description: null, cover_url: null, status: 'ongoing',
    author: null, year: null, tags: null, content_type: 'manga',
    in_library: false, library_id: undefined, alternatives: [],
    ...overrides,
  }
}

beforeEach(() => {
  vi.clearAllMocks()
  vi.mocked(api.listTitles).mockResolvedValue(emptyPage as never)
  vi.mocked(api.getTrending).mockResolvedValue([])
  vi.mocked(api.getTrendingManga).mockResolvedValue([])
  vi.mocked(api.getTrendingManhwa).mockResolvedValue([])
  vi.mocked(api.getTrendingManhua).mockResolvedValue([])
  vi.mocked(api.getTrendingAdultManhwa).mockResolvedValue([])
  vi.mocked(api.getNewReleases).mockResolvedValue([])
  vi.mocked(api.getContinueReading).mockResolvedValue([])
})

afterEach(() => {
  vi.useRealTimers()
})

describe('useHome', () => {
  it('loads trending on mount', async () => {
    const item = makeTrendingResult({ cover_url: 'https://cdn.example.com/cover.jpg', in_library: false })
    vi.mocked(api.getTrendingManga).mockResolvedValue([item] as never)

    const { result } = renderHook(() => useHome())
    await waitFor(() => expect(result.current.trendingLoading).toBe(false))
    expect(result.current.trending).toHaveLength(1)
    expect(result.current.trending[0].title).toBe('Berserk')
  })

  it('filters in-library titles from trending', async () => {
    vi.mocked(api.getTrending).mockResolvedValue([
      makeTrendingResult({ cover_url: 'https://cdn.example.com/1.jpg', in_library: false }),
      makeTrendingResult({ id: 't2', title: 'One Piece', cover_url: 'https://cdn.example.com/2.jpg', in_library: true }),
    ] as never)

    const { result } = renderHook(() => useHome())
    await waitFor(() => expect(result.current.trendingLoading).toBe(false))
    expect(result.current.trending.every((r) => !r.in_library)).toBe(true)
    expect(result.current.trending.some((r) => r.title === 'One Piece')).toBe(false)
  })

  it('stops cover poll once all items have a cover_url', async () => {
    vi.useFakeTimers()
    const withCover = makeTrendingResult({ cover_url: 'https://cdn.example.com/cover.jpg' })
    vi.mocked(api.getTrending).mockResolvedValue([withCover] as never)

    renderHook(() => useHome())
    await act(async () => { await Promise.resolve() })
    await act(async () => { await Promise.resolve() })

    const callsBefore = (api.getTrending as ReturnType<typeof vi.fn>).mock.calls.length

    await act(async () => {
      vi.advanceTimersByTime(5000)
      await Promise.resolve()
    })

    // No poll should have fired since all covers are present
    expect((api.getTrending as ReturnType<typeof vi.fn>).mock.calls.length).toBe(callsBefore)
  })

  it('trendingLoading is true initially and false after fetch', async () => {
    const { result } = renderHook(() => useHome())
    expect(result.current.trendingLoading).toBe(true)
    await waitFor(() => expect(result.current.trendingLoading).toBe(false))
  })
  it('refreshLibrary re-fetches listTitles', async () => {
    const { result } = renderHook(() => useHome())
    await waitFor(() => expect(result.current.isLoading).toBe(false))
    const callsBefore = (api.listTitles as ReturnType<typeof vi.fn>).mock.calls.length
    await act(async () => { result.current.refreshLibrary() })
    expect((api.listTitles as ReturnType<typeof vi.fn>).mock.calls.length).toBeGreaterThan(callsBefore)
  })

  describe('trending lanes (ADR 0032)', () => {
    it('fetches all 4 lanes independently on mount', async () => {
      const { result } = renderHook(() => useHome())
      await waitFor(() => expect(result.current.trendingMangaLoading).toBe(false))
      expect(api.getTrendingManga).toHaveBeenCalledTimes(1)
      expect(api.getTrendingManhwa).toHaveBeenCalledTimes(1)
      expect(api.getTrendingManhua).toHaveBeenCalledTimes(1)
    })

    it('trendingManga populated from getTrendingManga', async () => {
      const item = makeTrendingResult({ title: 'Berserk', content_type: 'manga', cover_url: 'https://cdn.test/1.jpg' })
      vi.mocked(api.getTrendingManga).mockResolvedValue([item] as never)
      const { result } = renderHook(() => useHome())
      await waitFor(() => expect(result.current.trendingMangaLoading).toBe(false))
      expect(result.current.trendingManga).toHaveLength(1)
      expect(result.current.trendingManga[0].title).toBe('Berserk')
    })

    it('trendingManhwa populated from getTrendingManhwa', async () => {
      const item = makeTrendingResult({ title: 'Solo Leveling', content_type: 'manhwa', cover_url: 'https://cdn.test/2.jpg' })
      vi.mocked(api.getTrendingManhwa).mockResolvedValue([item] as never)
      const { result } = renderHook(() => useHome())
      await waitFor(() => expect(result.current.trendingManhwaLoading).toBe(false))
      expect(result.current.trendingManhwa[0].title).toBe('Solo Leveling')
    })

    it('trendingManhua populated from getTrendingManhua', async () => {
      const item = makeTrendingResult({ title: 'Martial Peak', content_type: 'manhua', cover_url: 'https://cdn.test/3.jpg' })
      vi.mocked(api.getTrendingManhua).mockResolvedValue([item] as never)
      const { result } = renderHook(() => useHome())
      await waitFor(() => expect(result.current.trendingManhwaLoading).toBe(false))
      expect(result.current.trendingManhua[0].title).toBe('Martial Peak')
    })

    it('one lane failing does not affect other lanes', async () => {
      vi.mocked(api.getTrendingManga).mockRejectedValue(new Error('MU down'))
      const item = makeTrendingResult({ title: 'Solo Leveling', content_type: 'manhwa', cover_url: 'https://cdn.test/2.jpg' })
      vi.mocked(api.getTrendingManhwa).mockResolvedValue([item] as never)
      const { result } = renderHook(() => useHome())
      await waitFor(() => expect(result.current.trendingManhwaLoading).toBe(false))
      expect(result.current.trendingManga).toHaveLength(0)
      expect(result.current.trendingManhwa).toHaveLength(1)
    })

    it('filters in-library titles from each lane', async () => {
      vi.mocked(api.getTrendingManga).mockResolvedValue([
        makeTrendingResult({ cover_url: 'https://cdn.test/1.jpg', in_library: false }),
        makeTrendingResult({ id: 't2', cover_url: 'https://cdn.test/2.jpg', in_library: true }),
      ] as never)
      const { result } = renderHook(() => useHome())
      await waitFor(() => expect(result.current.trendingMangaLoading).toBe(false))
      expect(result.current.trendingManga.every((r) => !r.in_library)).toBe(true)
    })
  })

})