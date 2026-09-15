import { beforeEach, describe, expect, it, vi } from 'vitest'

vi.mock('./api', () => ({
  api: { me: vi.fn() },
  getToken: vi.fn(),
}))

import { SetupStore } from './setup.svelte'
import { api, getToken } from './api'
import { router } from './router.svelte'
import { ROUTES } from './routes'

beforeEach(() => {
  vi.clearAllMocks()
  vi.spyOn(router, 'navigate').mockImplementation(() => {})
})

describe('SetupStore', () => {
  it('starts on step 1', () => {
    vi.mocked(getToken).mockReturnValue(null)
    const store = new SetupStore()
    expect(store.step).toBe(1)
  })

  it('goToStep2 advances to step 2', () => {
    vi.mocked(getToken).mockReturnValue(null)
    const store = new SetupStore()
    store.goToStep2()
    expect(store.step).toBe(2)
  })

  it('redirects to home when token valid — setup already complete', async () => {
    vi.mocked(getToken).mockReturnValue('valid-token')
    vi.mocked(api.me).mockResolvedValue({ id: '1', username: 'a', role: 'admin', allow_explicit: false })
    new SetupStore()
    await vi.waitFor(() => expect(router.navigate).toHaveBeenCalledWith(ROUTES.home, { replace: true }))
  })

  it('stays on setup when token invalid (server wiped)', async () => {
    vi.mocked(getToken).mockReturnValue('stale-token')
    vi.mocked(api.me).mockRejectedValue(new Error('401'))
    new SetupStore()
    await new Promise((r) => setTimeout(r, 20))
    expect(router.navigate).not.toHaveBeenCalledWith(ROUTES.home, { replace: true })
  })

  it('no token — no me() call', () => {
    vi.mocked(getToken).mockReturnValue(null)
    new SetupStore()
    expect(api.me).not.toHaveBeenCalled()
  })
})
