import { renderHook, act, waitFor } from '@testing-library/react'
import { vi, describe, it, expect, beforeEach } from 'vitest'
import React from 'react'
import { MemoryRouter } from 'react-router-dom'

vi.mock('@/api', () => ({
  api: {
    getSettings: vi.fn(),
    saveSettings: vi.fn(),
  },
  clearToken: vi.fn(),
  isAdmin: vi.fn(),
}))

import { useSettings } from './useSettings'
import { api, clearToken, isAdmin } from '@/api'
beforeEach(async () => {
  await allure.epic('Settings')
  await allure.feature('Config')
})

const mockSettings = { workers: 3, reader_mode: 'paged' }

function wrapper({ children }: { children: React.ReactNode }) {
  return React.createElement(MemoryRouter, null, children)
}

beforeEach(() => {
  vi.clearAllMocks()
  vi.mocked(isAdmin).mockReturnValue(true)
  vi.mocked(api.getSettings).mockResolvedValue(mockSettings as never)
  vi.mocked(api.saveSettings).mockResolvedValue(mockSettings as never)
})

describe('useSettings', () => {
  it('loads settings on mount', async () => {
    const { result } = renderHook(() => useSettings(), { wrapper })
    await waitFor(() => expect(result.current.isLoading).toBe(false))
    expect(result.current.settings).toEqual(mockSettings)
  })

  it('defaults to library tab for admins', async () => {
    const { result } = renderHook(() => useSettings(), { wrapper })
    expect(result.current.tab).toBe('library')
  })

  it('defaults to account tab for non-admins', async () => {
    vi.mocked(isAdmin).mockReturnValue(false)
    const { result } = renderHook(() => useSettings(), { wrapper })
    expect(result.current.tab).toBe('account')
  })

  it('handleSave patches settings', async () => {
    const updated = { workers: 5, reader_mode: 'scroll' }
    vi.mocked(api.saveSettings).mockResolvedValue(updated as never)
    const { result } = renderHook(() => useSettings(), { wrapper })
    await waitFor(() => expect(result.current.isLoading).toBe(false))
    await act(async () => { await result.current.handleSave({ download_workers: 5 }) })
    expect(api.saveSettings).toHaveBeenCalledWith({ download_workers: 5 })
    expect(result.current.settings).toEqual(updated)
  })

  it('logout calls clearToken', async () => {
    const { result } = renderHook(() => useSettings(), { wrapper })
    act(() => { result.current.logout() })
    expect(clearToken).toHaveBeenCalled()
  })
})
