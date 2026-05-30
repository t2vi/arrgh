import { renderHook, waitFor } from '@testing-library/react'
import { vi, describe, it, expect, beforeEach } from 'vitest'
import React from 'react'
import { MemoryRouter } from 'react-router-dom'

const mockNavigate = vi.fn()

vi.mock('react-router-dom', async () => {
  const actual = await vi.importActual('react-router-dom')
  return { ...actual, useNavigate: () => mockNavigate }
})

vi.mock('@/api', () => ({
  api: { me: vi.fn() },
  getToken: vi.fn(),
}))

import { useSetup } from './useSetup'
import { api, getToken } from '@/api'

function wrapper({ children }: { children: React.ReactNode }) {
  return React.createElement(MemoryRouter, null, children)
}

beforeEach(() => {
  vi.clearAllMocks()
})

describe('useSetup', () => {
  it('starts on step 1', () => {
    vi.mocked(getToken).mockReturnValue(null)
    const { result } = renderHook(() => useSetup(), { wrapper })
    expect(result.current.step).toBe(1)
  })

  it('goToStep2 advances to step 2', async () => {
    vi.mocked(getToken).mockReturnValue(null)
    const { result } = renderHook(() => useSetup(), { wrapper })
    result.current.goToStep2()
    await waitFor(() => expect(result.current.step).toBe(2))
  })

  it('redirects to home when token valid — setup already complete', async () => {
    vi.mocked(getToken).mockReturnValue('valid-token')
    vi.mocked(api.me).mockResolvedValue({} as never)
    renderHook(() => useSetup(), { wrapper })
    await waitFor(() => expect(mockNavigate).toHaveBeenCalledWith('/', { replace: true }))
  })

  it('stays on setup when token invalid (server wiped)', async () => {
    vi.mocked(getToken).mockReturnValue('stale-token')
    vi.mocked(api.me).mockRejectedValue(new Error('401'))
    renderHook(() => useSetup(), { wrapper })
    // me() rejected → no navigation to home
    await new Promise((r) => setTimeout(r, 50))
    expect(mockNavigate).not.toHaveBeenCalledWith('/', { replace: true })
  })

  it('no token — no me() call', () => {
    vi.mocked(getToken).mockReturnValue(null)
    renderHook(() => useSetup(), { wrapper })
    expect(api.me).not.toHaveBeenCalled()
  })
})
