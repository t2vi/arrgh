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
      vi.fn().mockResolvedValue({
        ok: true,
        status: 200,
        json: async () => ({ current: '1.2.3', latest: null, release_url: null }),
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

    expect(screen.queryByText('*ARRgh')).not.toBeInTheDocument()
  })
})
