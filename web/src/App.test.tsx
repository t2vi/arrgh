import { render, screen, waitFor } from '@testing-library/react'
import { MemoryRouter } from 'react-router-dom'
import { vi, describe, it, expect, beforeEach } from 'vitest'
import React from 'react'

vi.mock('@/api', () => ({
  api: {
    authStatus: vi.fn(),
    me: vi.fn(),
  },
  getToken: vi.fn(() => null),
  clearToken: vi.fn(),
}))

import App from './App'
import { api } from '@/api'

function renderAt(path: string) {
  return render(
    <MemoryRouter initialEntries={[path]}>
      <App />
    </MemoryRouter>
  )
}

beforeEach(() => {
  vi.clearAllMocks()
})

describe('SetupGuard (/login)', () => {
  it('redirects to /setup when needs_setup is true', async () => {
    vi.mocked(api.authStatus).mockResolvedValue({ needs_setup: true, is_registered: false } as never)
    renderAt('/login')
    await waitFor(() =>
      expect(document.body.textContent).toMatch(/create your account/i)
    )
  })

  it('renders login when needs_setup is false', async () => {
    vi.mocked(api.authStatus).mockResolvedValue({ needs_setup: false, is_registered: true } as never)
    renderAt('/login')
    await waitFor(() =>
      expect(document.body.textContent).toMatch(/log in|sign in|password/i)
    )
  })

  it('shows spinner while authStatus is pending', async () => {
    let resolve!: (v: unknown) => void
    vi.mocked(api.authStatus).mockImplementation(() => new Promise((r) => { resolve = r }))
    const { container } = renderAt('/login')
    expect(container.querySelector('.animate-spin')).toBeTruthy()
    resolve({ needs_setup: false, is_registered: true })
  })
})
