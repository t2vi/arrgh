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
    addTitle: vi.fn(),
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
  source: 'mangaupdates',
  is_explicit: false,
}

const anilistResult = {
  ...mockResult,
  source: 'anilist',
  mangaupdates_id: 'al-999',
  content_type: 'manhwa',
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
    vi.mocked(api.addTitle).mockResolvedValue(mockManga as never)
    const { result } = renderHook(() => useDiscover(), { wrapper })
    await act(async () => {
      await result.current.handleAdd(mockResult)
    })
    expect(mockNavigate).toHaveBeenCalledWith('/library')
  })

  // 502 error message should describe multi-source failure, not single source
  it('sets searchError on 502 with generic discovery message', async () => {
    vi.mocked(api.searchManga).mockRejectedValue(new Error('502 Bad Gateway'))
    const { result } = renderHook(() => useDiscover(), { wrapper })
    act(() => { result.current.setQuery('test') })
    await act(async () => { result.current.submit() })
    await waitFor(() => {
      expect(result.current.searchError).toBeTruthy()
      // Should no longer say "MangaUpdates" specifically — fan-out covers all sources
      expect(result.current.searchError).not.toMatch(/^MangaUpdates search failed/)
    })
  })

  it('tracks added manga by mangaupdates_id (MU source)', async () => {
    const mockManga = { id: 'manga-99', title: 'Test' }
    vi.mocked(api.addTitle).mockResolvedValue(mockManga as never)
    const { result } = renderHook(() => useDiscover(), { wrapper })
    await act(async () => { await result.current.handleAdd(mockResult) })
    expect(result.current.added.get('mu-001')).toBe('manga-99')
  })

  it('tracks added manga by mangaupdates_id for non-MU sources', async () => {
    const mockManga = { id: 'manga-anilist', title: 'Test Manhwa' }
    vi.mocked(api.addTitle).mockResolvedValue(mockManga as never)
    const { result } = renderHook(() => useDiscover(), { wrapper })
    await act(async () => { await result.current.handleAdd(anilistResult) })
    expect(result.current.added.get('al-999')).toBe('manga-anilist')
  })

  // addTitle must send source + source_id so .NET can route to the correct metadata store
  it('addTitle is called with source field', async () => {
    vi.mocked(api.addTitle).mockResolvedValue({ id: 'manga-1' } as never)
    const { result } = renderHook(() => useDiscover(), { wrapper })
    await act(async () => { await result.current.handleAdd(mockResult) })
    expect(api.addTitle).toHaveBeenCalledWith(
      expect.objectContaining({ source: 'mangaupdates' })
    )
  })

  it('addTitle for anilist result sends anilist source', async () => {
    vi.mocked(api.addTitle).mockResolvedValue({ id: 'manga-2' } as never)
    const { result } = renderHook(() => useDiscover(), { wrapper })
    await act(async () => { await result.current.handleAdd(anilistResult) })
    expect(api.addTitle).toHaveBeenCalledWith(
      expect.objectContaining({ source: 'anilist' })
    )
  })

  it('addingId is set to mangaupdates_id while add is in flight', async () => {
    let resolveAdd!: (v: unknown) => void
    vi.mocked(api.addTitle).mockImplementation(
      () => new Promise((res) => { resolveAdd = res })
    )
    const { result } = renderHook(() => useDiscover(), { wrapper })
    act(() => { result.current.handleAdd(mockResult) })
    await waitFor(() => expect(result.current.addingId).toBe('mu-001'))
    act(() => resolveAdd({ id: 'manga-1' }))
    await waitFor(() => expect(result.current.addingId).toBeNull())
  })

  it('clears addingId after add completes', async () => {
    vi.mocked(api.addTitle).mockResolvedValue({ id: 'manga-1' } as never)
    const { result } = renderHook(() => useDiscover(), { wrapper })
    await act(async () => { await result.current.handleAdd(mockResult) })
    expect(result.current.addingId).toBeNull()
  })

  it('sets addError when addTitle throws', async () => {
    vi.mocked(api.addTitle).mockRejectedValue(new Error('Not found'))
    const { result } = renderHook(() => useDiscover(), { wrapper })
    await act(async () => { await result.current.handleAdd(mockResult) })
    await waitFor(() => expect(result.current.addError).toBe('Not found'))
  })

  // ── Content-type filter ────────────────────────────────────────────────────
  // Fan-out returns mixed content types; users need to filter down to one type.

  it('availableTypes derived from all result content_type values', async () => {
    const mixed = [
      { ...mockResult, content_type: 'manga' },
      { ...anilistResult, content_type: 'manhwa' },
      { ...mockResult, mangaupdates_id: 'nu-1', source: 'novelupdates', content_type: 'novel' },
    ]
    vi.mocked(api.searchManga).mockResolvedValue(mixed as never)
    const { result } = renderHook(() => useDiscover(), { wrapper })
    act(() => { result.current.setQuery('test') })
    await act(async () => { result.current.submit() })
    await waitFor(() => expect(result.current.availableTypes).toEqual(new Set(['manga', 'manhwa', 'novel'])))
  })

  it('filteredData returns all results when contentTypeFilter is undefined', async () => {
    const mixed = [
      { ...mockResult, content_type: 'manga' },
      { ...anilistResult, content_type: 'manhwa' },
    ]
    vi.mocked(api.searchManga).mockResolvedValue(mixed as never)
    const { result } = renderHook(() => useDiscover(), { wrapper })
    act(() => { result.current.setQuery('test') })
    await act(async () => { result.current.submit() })
    await waitFor(() => expect(result.current.filteredData).toHaveLength(2))
  })

  it('filteredData returns only matching content_type when filter set', async () => {
    const mixed = [
      { ...mockResult, content_type: 'manga' },
      { ...anilistResult, content_type: 'manhwa' },
    ]
    vi.mocked(api.searchManga).mockResolvedValue(mixed as never)
    const { result } = renderHook(() => useDiscover(), { wrapper })
    act(() => { result.current.setQuery('test') })
    await act(async () => { result.current.submit() })
    await waitFor(() => expect(result.current.data).toHaveLength(2))
    act(() => { result.current.setContentTypeFilter('manhwa') })
    await waitFor(() => {
      expect(result.current.filteredData).toHaveLength(1)
      expect(result.current.filteredData![0].content_type).toBe('manhwa')
    })
  })

  it('setContentTypeFilter resets to undefined when called with current value (toggle off)', async () => {
    vi.mocked(api.searchManga).mockResolvedValue([mockResult] as never)
    const { result } = renderHook(() => useDiscover(), { wrapper })
    act(() => { result.current.setQuery('test') })
    await act(async () => { result.current.submit() })
    await waitFor(() => expect(result.current.data).toBeDefined())
    act(() => { result.current.setContentTypeFilter('manga') })
    await waitFor(() => expect(result.current.contentTypeFilter).toBe('manga'))
    act(() => { result.current.setContentTypeFilter('manga') })
    await waitFor(() => expect(result.current.contentTypeFilter).toBeUndefined())
  })

  it('availableTypes includes hentai when explicit results present', async () => {
    const explicitResult = { ...mockResult, mangaupdates_id: 'eh-1', source: 'ehentai', content_type: 'hentai', is_explicit: true }
    vi.mocked(api.searchManga).mockResolvedValue([mockResult, explicitResult] as never)
    const { result } = renderHook(() => useDiscover(), { wrapper })
    act(() => { result.current.setQuery('test') })
    await act(async () => { result.current.submit() })
    await waitFor(() => expect(result.current.availableTypes?.has('hentai')).toBe(true))
  })

  it('resets contentTypeFilter when new search is submitted', async () => {
    vi.mocked(api.searchManga).mockResolvedValue([mockResult] as never)
    const { result } = renderHook(() => useDiscover(), { wrapper })
    act(() => { result.current.setQuery('first') })
    await act(async () => { result.current.submit() })
    await waitFor(() => expect(result.current.data).toBeDefined())
    act(() => { result.current.setContentTypeFilter('manga') })
    act(() => { result.current.setQuery('second') })
    await act(async () => { result.current.submit() })
    await waitFor(() => expect(result.current.contentTypeFilter).toBeUndefined())
  })
})
