import { renderHook, act, waitFor } from '@testing-library/react'
import { vi, describe, it, expect, beforeEach, afterEach } from 'vitest'

vi.mock('@/api', () => ({
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

import { useHome } from './useHome'
import { api } from '@/api'

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
    vi.mocked(api.getTrendingManga).mockResolvedValue([
      makeTrendingResult({ cover_url: 'https://cdn.example.com/1.jpg', in_library: false }),
      makeTrendingResult({ id: 't2', title: 'One Piece', cover_url: 'https://cdn.example.com/2.jpg', in_library: true }),
    ] as never)

    const { result } = renderHook(() => useHome())
    await waitFor(() => expect(result.current.trendingLoading).toBe(false))
    expect(result.current.trending.every((r) => !r.in_library)).toBe(true)
    expect(result.current.trending.some((r) => r.title === 'One Piece')).toBe(false)
  })

  it('trendingLoading is true initially and false after fetch', async () => {
    const { result } = renderHook(() => useHome())
    expect(result.current.trendingLoading).toBe(true)
    await waitFor(() => expect(result.current.trendingLoading).toBe(false))
  })
})
