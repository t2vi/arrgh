import { render, screen } from '@testing-library/svelte'
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import App from './App.svelte'
import { router } from './lib/router.svelte'
import { ROUTES } from './lib/routes'

describe('App shell', () => {
  beforeEach(() => {
    router.navigate(ROUTES.home, { replace: true })
    vi.stubGlobal(
      'fetch',
      vi.fn().mockImplementation((input: RequestInfo | URL) => {
        const url = input.toString()
        const body = url.includes('/api/version')
          ? { current: '1.2.3', latest: null, release_url: null }
          : url.includes('/api/titles')
            ? { items: [], total: 0, page: 1, limit: 20 }
            : []
        return Promise.resolve({ ok: true, status: 200, json: async () => body })
      }),
    )
  })

  afterEach(() => {
    vi.unstubAllGlobals()
  })

  it('renders the sidebar nav and fetches /api/version', async () => {
    render(App)

    expect(screen.getByText('*ARRgh')).toBeInTheDocument()
    expect(screen.getByText('Library')).toBeInTheDocument()
    expect(screen.getByText('Discover')).toBeInTheDocument()
    expect(screen.getByText('Downloads')).toBeInTheDocument()
    expect(screen.getByText('Settings')).toBeInTheDocument()

    expect(await screen.findByText('v1.2.3')).toBeInTheDocument()
    const [url] = vi.mocked(fetch).mock.calls[0]
    expect(url.toString()).toContain('/api/version')
  })

  it('renders the login page without the sidebar shell', () => {
    router.navigate(ROUTES.login, { replace: true })
    render(App)

    expect(document.querySelector('[data-nav]')).not.toBeInTheDocument()
  })

  // spec: 003/FR-007, 001/FR-003
  it('navigates to login when an arrgh:unauthorized event is observed', async () => {
    render(App)
    expect(router.path).not.toBe(ROUTES.login)

    window.dispatchEvent(new Event('arrgh:unauthorized'))

    expect(router.path).toBe(ROUTES.login)
  })
})
