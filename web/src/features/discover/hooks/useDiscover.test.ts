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
    listSources: vi.fn(),
    searchManga: vi.fn(),
    addManga: vi.fn(),
  },
}))

import { useDiscover } from './useDiscover'
import { api } from '@/api'

const mockSources = [
  { id: '1', name: 'MangaDex', base_url: '', has_api_key: false, content_types: ['manga', 'manhwa'], enabled: true },
  { id: '2', name: 'Disabled', base_url: '', has_api_key: false, content_types: ['manhua'], enabled: false },
]

function wrapper({ children }: { children: React.ReactNode }) {
  return React.createElement(MemoryRouter, null, children)
}

beforeEach(() => {
  vi.clearAllMocks()
  mockNavigate.mockReset()
  vi.mocked(api.listSources).mockResolvedValue(mockSources as never)
  vi.mocked(api.searchManga).mockResolvedValue([])
})

describe('useDiscover', () => {
  it('populates availableContentTypes from enabled sources only', async () => {
    const { result } = renderHook(() => useDiscover(), { wrapper })
    await waitFor(() => expect(result.current.availableContentTypes.size).toBeGreaterThan(0))
    expect(result.current.availableContentTypes.has('manga')).toBe(true)
    expect(result.current.availableContentTypes.has('manhwa')).toBe(true)
    expect(result.current.availableContentTypes.has('manhua')).toBe(false)
  })

  it('submit triggers searchManga with current query', async () => {
    const { result } = renderHook(() => useDiscover(), { wrapper })
    await waitFor(() => expect(result.current.availableContentTypes.size).toBeGreaterThan(0))
    act(() => { result.current.setQuery('naruto') })
    await act(async () => { result.current.submit() })
    await waitFor(() => expect(api.searchManga).toHaveBeenCalledWith('naruto', undefined))
  })

  it('submit does nothing when query is blank', async () => {
    const { result } = renderHook(() => useDiscover(), { wrapper })
    await act(async () => { result.current.submit() })
    expect(api.searchManga).not.toHaveBeenCalled()
  })

  it('resets contentType if selected type is not in available set', async () => {
    // Start with manhwa selected
    const { result } = renderHook(() => useDiscover(), { wrapper })
    await waitFor(() => expect(result.current.availableContentTypes.size).toBeGreaterThan(0))
    act(() => { result.current.setContentType('manhwa') })

    // Now sources change to exclude manhwa
    vi.mocked(api.listSources).mockResolvedValue([
      { id: '1', name: 'Src', base_url: '', has_api_key: false, content_types: ['manga'], enabled: true },
    ] as never)
    // Re-render a fresh hook with the new sources
    const { result: result2 } = renderHook(() => useDiscover(), { wrapper })
    await waitFor(() => expect(result2.current.availableContentTypes.has('manhwa')).toBe(false))
    expect(result2.current.contentType).toBeUndefined()
  })

  it('navigates to library after handleAdd succeeds', async () => {
    const mockManga = { id: 'manga-1', title: 'Test', source: 'src', source_id: 'sid' }
    vi.mocked(api.addManga).mockResolvedValue(mockManga as never)
    const { result } = renderHook(() => useDiscover(), { wrapper })
    await waitFor(() => expect(result.current.availableContentTypes.size).toBeGreaterThan(0))
    await act(async () => {
      await result.current.handleAdd({
        id: 'sid', title: 'Test', source: 'src', source_id: 'sid',
        description: null, cover_url: null, status: 'ongoing',
        author: null, year: null, tags: null, content_type: 'manga', alternatives: [],
      })
    })
    expect(mockNavigate).toHaveBeenCalledWith('/library')
  })

  it('sets searchError on 502', async () => {
    vi.mocked(api.searchManga).mockRejectedValue(new Error('502 Bad Gateway'))
    const { result } = renderHook(() => useDiscover(), { wrapper })
    await waitFor(() => expect(result.current.availableContentTypes.size).toBeGreaterThan(0))
    act(() => { result.current.setQuery('test') })
    await act(async () => { result.current.submit() })
    await waitFor(() => expect(result.current.searchError).toMatch(/sources failed/))
  })
})
