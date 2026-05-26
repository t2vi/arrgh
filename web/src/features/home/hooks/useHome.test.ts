import { renderHook, act, waitFor } from '@testing-library/react'
import { vi, describe, it, expect, beforeEach, afterEach } from 'vitest'

vi.mock('@/api', () => ({
  api: {
    listTitles: vi.fn(),
    getTrending: vi.fn(),
    getNewReleases: vi.fn(),
    getContinueReading: vi.fn(),
  },
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
  vi.mocked(api.getTrending).mockResolvedValue([])
  vi.mocked(api.getNewReleases).mockResolvedValue([])
  vi.mocked(api.getContinueReading).mockResolvedValue([])
})

afterEach(() => {
  vi.useRealTimers()
})

describe('useHome', () => {
  it('loads trending on mount', async () => {
    const item = makeTrendingResult({ cover_url: 'https://cdn.example.com/cover.jpg', in_library: false })
    vi.mocked(api.getTrending).mockResolvedValue([item] as never)

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

  it('polls trending every 5s while any item has no cover_url', async () => {
    vi.useFakeTimers()
    const missingCover = makeTrendingResult({ cover_url: null })
    vi.mocked(api.getTrending).mockResolvedValue([missingCover] as never)

    const { result } = renderHook(() => useHome())
    await act(async () => { await Promise.resolve() })
    await act(async () => { await Promise.resolve() })

    const callsBefore = (api.getTrending as ReturnType<typeof vi.fn>).mock.calls.length

    await act(async () => {
      vi.advanceTimersByTime(5000)
      await Promise.resolve()
    })

    expect((api.getTrending as ReturnType<typeof vi.fn>).mock.calls.length).toBeGreaterThan(callsBefore)
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

  it('does not poll while trendingLoading is true', async () => {
    vi.useFakeTimers()
    let resolveFirst!: (v: unknown) => void
    vi.mocked(api.getTrending).mockReturnValue(
      new Promise((r) => { resolveFirst = r }) as never
    )

    renderHook(() => useHome())

    await act(async () => {
      vi.advanceTimersByTime(5000)
      await Promise.resolve()
    })

    // Only the initial fetch; poll must not have fired while loading
    expect((api.getTrending as ReturnType<typeof vi.fn>).mock.calls.length).toBe(1)
    resolveFirst([])
  })

  it('trendingLoading is true initially and false after fetch', async () => {
    const { result } = renderHook(() => useHome())
    expect(result.current.trendingLoading).toBe(true)
    await waitFor(() => expect(result.current.trendingLoading).toBe(false))
  })
})
