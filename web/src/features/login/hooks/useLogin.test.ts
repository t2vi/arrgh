import { renderHook, act, waitFor } from '@testing-library/react'
import { vi, describe, it, expect, beforeEach } from 'vitest'
import React from 'react'
import { MemoryRouter } from 'react-router-dom'

vi.mock('@/api', () => ({
  api: { login: vi.fn() },
  setToken: vi.fn(),
}))

import { useLogin } from './useLogin'
import { api, setToken } from '@/api'

function wrapper({ children }: { children: React.ReactNode }) {
  return React.createElement(MemoryRouter, null, children)
}

const mockEvent = { preventDefault: vi.fn() } as unknown as React.FormEvent

beforeEach(() => {
  vi.clearAllMocks()
})

describe('useLogin', () => {
  it('starts with empty fields and no error', () => {
    const { result } = renderHook(() => useLogin(), { wrapper })
    expect(result.current.username).toBe('')
    expect(result.current.password).toBe('')
    expect(result.current.error).toBe('')
    expect(result.current.loading).toBe(false)
  })

  it('calls api.login and setToken on successful submit', async () => {
    vi.mocked(api.login).mockResolvedValue({
      token: 'tok', username: 'alice', user_id: '1', role: 'admin', allow_explicit: false,
    } as never)
    const { result } = renderHook(() => useLogin(), { wrapper })
    act(() => { result.current.setUsername('alice'); result.current.setPassword('pw') })
    await act(async () => { await result.current.handleSubmit(mockEvent) })
    expect(api.login).toHaveBeenCalledWith('alice', 'pw')
    expect(setToken).toHaveBeenCalledWith('tok', 'alice', 'admin', false)
  })

  it('sets error on failed login', async () => {
    vi.mocked(api.login).mockRejectedValue(new Error('401'))
    const { result } = renderHook(() => useLogin(), { wrapper })
    act(() => { result.current.setUsername('bad'); result.current.setPassword('bad') })
    await act(async () => { await result.current.handleSubmit(mockEvent) })
    await waitFor(() => expect(result.current.error).toMatch(/Invalid/))
    expect(result.current.loading).toBe(false)
  })

  it('clears loading after successful login', async () => {
    vi.mocked(api.login).mockResolvedValue({
      token: 't', username: 'u', user_id: '1', role: 'member', allow_explicit: false,
    } as never)
    const { result } = renderHook(() => useLogin(), { wrapper })
    act(() => { result.current.setUsername('u'); result.current.setPassword('p') })
    await act(async () => { await result.current.handleSubmit(mockEvent) })
    expect(result.current.loading).toBe(false)
  })
})
