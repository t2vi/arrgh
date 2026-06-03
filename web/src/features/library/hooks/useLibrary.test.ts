import { renderHook, act, waitFor } from '@testing-library/react'
import { vi, describe, it, expect, beforeEach, afterEach } from 'vitest'

vi.mock('@/api', () => ({
  api: {
    listTitles: vi.fn(),
    removeTitle: vi.fn(),
    getSyncLog: vi.fn().mockResolvedValue([]),
  },
}))

import { useLibrary } from './useLibrary'
import { api } from '@/api'
beforeEach(async () => {
  await allure.epic('Library')
  await allure.feature('Library List')
})

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

describe('useLibrary', () => {
  it('fetches on mount and sets data', async () => {
    const { result } = renderHook(() => useLibrary())
    await waitFor(() => expect(result.current.loading).toBe(false))
    expect(api.listTitles).toHaveBeenCalledTimes(1)
    expect(result.current.data?.items).toHaveLength(1)
  })

  it('computes totalPages correctly', async () => {
    vi.mocked(api.listTitles).mockResolvedValue({ items: [], total: 45, page: 1, limit: 20 } as never)
    const { result } = renderHook(() => useLibrary())
    await waitFor(() => expect(result.current.loading).toBe(false))
    expect(result.current.totalPages).toBe(3)
  })

  it('handleRemove calls removeManga then refetches', async () => {
    const { result } = renderHook(() => useLibrary())
    await waitFor(() => expect(result.current.loading).toBe(false))
    await act(() => result.current.handleRemove('m1', false))
    expect(api.removeTitle).toHaveBeenCalledWith('m1', false)
    expect(api.listTitles).toHaveBeenCalledTimes(2)
  })

  it('sets removingId during removal then clears it', async () => {
    let resolve!: () => void
    vi.mocked(api.removeTitle).mockReturnValue(new Promise<void>((r) => { resolve = r }) as never)
    const { result } = renderHook(() => useLibrary())
    await waitFor(() => expect(result.current.loading).toBe(false))

    // Start removal without awaiting
    act(() => { result.current.handleRemove('m1', false) })
    expect(result.current.removingId).toBe('m1')
    resolve()
    await waitFor(() => expect(result.current.removingId).toBeNull())
  })

  it('sort defaults to "recent"', async () => {
    const { result } = renderHook(() => useLibrary())
    expect(result.current.sort).toBe('recent')
  })

  it('setSort resets page to 1 and refetches with new sort', async () => {
    const { result } = renderHook(() => useLibrary())
    await waitFor(() => expect(result.current.loading).toBe(false))
    vi.clearAllMocks()
    vi.mocked(api.listTitles).mockResolvedValue(mockPage as never)

    act(() => result.current.setSort('title_asc'))
    await waitFor(() => expect(api.listTitles).toHaveBeenCalled())
    expect(api.listTitles).toHaveBeenCalledWith(1, undefined, 'title_asc', undefined, undefined)
  })

  it('toggleContentType adds then removes value', async () => {
    const { result } = renderHook(() => useLibrary())
    await waitFor(() => expect(result.current.loading).toBe(false))

    act(() => result.current.toggleContentType('manga'))
    expect(result.current.contentTypes).toEqual(['manga'])

    act(() => result.current.toggleContentType('manga'))
    expect(result.current.contentTypes).toEqual([])
  })

  it('toggleContentType resets page to 1', async () => {
    const { result } = renderHook(() => useLibrary())
    await waitFor(() => expect(result.current.loading).toBe(false))
    act(() => result.current.setPage(3))

    act(() => result.current.toggleContentType('manhwa'))
    expect(result.current.page).toBe(1)
  })

  it('toggleStatus adds then removes value', async () => {
    const { result } = renderHook(() => useLibrary())
    await waitFor(() => expect(result.current.loading).toBe(false))

    act(() => result.current.toggleStatus('ongoing'))
    expect(result.current.statuses).toEqual(['ongoing'])

    act(() => result.current.toggleStatus('ongoing'))
    expect(result.current.statuses).toEqual([])
  })

  it('hasFilters is false with no filters, true when any active', async () => {
    const { result } = renderHook(() => useLibrary())
    await waitFor(() => expect(result.current.loading).toBe(false))
    expect(result.current.hasFilters).toBe(false)

    act(() => result.current.toggleContentType('manga'))
    expect(result.current.hasFilters).toBe(true)
  })

  it('clearFilters resets contentTypes and statuses and resets page', async () => {
    const { result } = renderHook(() => useLibrary())
    await waitFor(() => expect(result.current.loading).toBe(false))

    act(() => { result.current.toggleContentType('manga'); result.current.toggleStatus('ongoing') })
    expect(result.current.hasFilters).toBe(true)

    act(() => result.current.clearFilters())
    expect(result.current.contentTypes).toEqual([])
    expect(result.current.statuses).toEqual([])
    expect(result.current.hasFilters).toBe(false)
    expect(result.current.page).toBe(1)
  })

  it('fetches with contentType and status params when filters active', async () => {
    const { result } = renderHook(() => useLibrary())
    await waitFor(() => expect(result.current.loading).toBe(false))
    vi.clearAllMocks()
    vi.mocked(api.listTitles).mockResolvedValue(mockPage as never)

    act(() => { result.current.toggleContentType('manga'); result.current.toggleStatus('ongoing') })
    await waitFor(() => expect(api.listTitles).toHaveBeenCalled())
    expect(api.listTitles).toHaveBeenCalledWith(1, undefined, 'recent', ['manga'], ['ongoing'])
  })

  it('showFilters defaults false, setShowFilters toggles it', async () => {
    const { result } = renderHook(() => useLibrary())
    expect(result.current.showFilters).toBe(false)
    act(() => result.current.setShowFilters(true))
    expect(result.current.showFilters).toBe(true)
  })

  it('polls every 2s when a manga is syncing', async () => {
    vi.useFakeTimers()
    const syncingPage = {
      items: [{ id: 'm1', title: 'X', sync_status: 'syncing' }],
      total: 1, page: 1, limit: 20,
    }
    vi.mocked(api.listTitles).mockResolvedValue(syncingPage as never)

    const { result } = renderHook(() => useLibrary())
    // Flush the initial fetch promise
    await act(async () => { await Promise.resolve() })
    const callsBefore = (api.listTitles as ReturnType<typeof vi.fn>).mock.calls.length

    await act(async () => {
      vi.advanceTimersByTime(2000)
      await Promise.resolve()
    })
    expect((api.listTitles as ReturnType<typeof vi.fn>).mock.calls.length).toBeGreaterThan(callsBefore)
  })
})
