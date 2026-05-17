import { renderHook, act, waitFor } from '@testing-library/react'
import { vi, describe, it, expect, beforeEach, afterEach } from 'vitest'
import { useQueue } from './useQueue'

vi.mock('@/api', () => ({
  api: {
    getQueue: vi.fn(),
    removeFromQueue: vi.fn(),
    clearCompletedQueue: vi.fn(),
  },
}))

import { api } from '@/api'

const mockItems = [
  { id: '1', manga_title: 'Test', chapter_num: '1', status: 'done', error: null, chapter_id: 'c1', created_at: '', updated_at: '' },
  { id: '2', manga_title: 'Test', chapter_num: '2', status: 'pending', error: null, chapter_id: 'c2', created_at: '', updated_at: '' },
]

beforeEach(() => {
  vi.clearAllMocks()
  vi.mocked(api.getQueue).mockResolvedValue(mockItems as never)
})

afterEach(() => {
  vi.useRealTimers()
})

describe('useQueue', () => {
  it('fetches queue on mount', async () => {
    const { result } = renderHook(() => useQueue())
    await waitFor(() => expect(result.current.loading).toBe(false))
    expect(api.getQueue).toHaveBeenCalledTimes(1)
    expect(result.current.data).toHaveLength(2)
  })

  it('sorts by status order (pending before done)', async () => {
    const { result } = renderHook(() => useQueue())
    await waitFor(() => expect(result.current.data).toBeDefined())
    expect(result.current.data![0].status).toBe('pending')
    expect(result.current.data![1].status).toBe('done')
  })

  it('canClear true when completed items exist', async () => {
    const { result } = renderHook(() => useQueue())
    await waitFor(() => expect(result.current.loading).toBe(false))
    expect(result.current.canClear).toBe(true)
  })

  it('canClear false when only active items', async () => {
    vi.mocked(api.getQueue).mockResolvedValue([
      { id: '1', manga_title: 'T', chapter_num: '1', status: 'downloading', error: null, chapter_id: 'c1', created_at: '', updated_at: '' },
    ] as never)
    const { result } = renderHook(() => useQueue())
    await waitFor(() => expect(result.current.loading).toBe(false))
    expect(result.current.canClear).toBe(false)
  })

  it('handleRemove calls api and refetches', async () => {
    vi.mocked(api.removeFromQueue).mockResolvedValue(undefined as never)
    const { result } = renderHook(() => useQueue())
    await waitFor(() => expect(result.current.loading).toBe(false))
    await act(() => result.current.handleRemove('1'))
    expect(api.removeFromQueue).toHaveBeenCalledWith('1')
    expect(api.getQueue).toHaveBeenCalledTimes(2)
  })
})
