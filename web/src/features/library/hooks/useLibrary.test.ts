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
