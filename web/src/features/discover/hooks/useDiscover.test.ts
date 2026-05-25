import { renderHook, act, waitFor } from '@testing-library/react'
import { vi, describe, it, expect, beforeEach } from 'vitest'
import { MemoryRouter } from 'react-router-dom'
import React from 'react'

const mockNavigate = vi.fn()

vi.mock('react-router-dom', async (importOriginal) => {
  const mod = await importOriginal<typeof import('react-router-dom')>()
  return { ...mod, useNavigate: () => mockNavigate }
})

vi.mock('@/api', () => ({
  api: {
    searchManga: vi.fn(),
    addManga: vi.fn(),
  },
}))

import { useDiscover } from './useDiscover'
import { api } from '@/api'

function wrapper({ children }: { children: React.ReactNode }) {
  return React.createElement(MemoryRouter, null, children)
}

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
}

beforeEach(() => {
  vi.clearAllMocks()
  mockNavigate.mockReset()
  vi.mocked(api.searchManga).mockResolvedValue([])
})

describe('useDiscover', () => {
  it('submit triggers searchManga with current query', async () => {
    const { result } = renderHook(() => useDiscover(), { wrapper })
    act(() => { result.current.setQuery('naruto') })
    await act(async () => { result.current.submit() })
    await waitFor(() => expect(api.searchManga).toHaveBeenCalledWith('naruto'))
  })

  it('submit does nothing when query is blank', async () => {
    const { result } = renderHook(() => useDiscover(), { wrapper })
    await act(async () => { result.current.submit() })
    expect(api.searchManga).not.toHaveBeenCalled()
  })

  it('navigates to library after handleAdd succeeds', async () => {
    const mockManga = { id: 'manga-1', title: 'Test' }
    vi.mocked(api.addManga).mockResolvedValue(mockManga as never)
    const { result } = renderHook(() => useDiscover(), { wrapper })
    await act(async () => {
      await result.current.handleAdd(mockResult)
    })
    expect(mockNavigate).toHaveBeenCalledWith('/library')
  })

  it('sets searchError on 502', async () => {
    vi.mocked(api.searchManga).mockRejectedValue(new Error('502 Bad Gateway'))
    const { result } = renderHook(() => useDiscover(), { wrapper })
    act(() => { result.current.setQuery('test') })
    await act(async () => { result.current.submit() })
    await waitFor(() => expect(result.current.searchError).toMatch(/MangaUpdates/))
  })

  it('tracks added manga by mangaupdates_id', async () => {
    const mockManga = { id: 'manga-99', title: 'Test' }
    vi.mocked(api.addManga).mockResolvedValue(mockManga as never)
    const { result } = renderHook(() => useDiscover(), { wrapper })
    await act(async () => { await result.current.handleAdd(mockResult) })
    expect(result.current.added.get('mu-001')).toBe('manga-99')
  })
})
