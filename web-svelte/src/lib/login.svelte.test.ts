import { beforeEach, describe, expect, it, vi } from 'vitest'

vi.mock('./api', () => ({
  api: { login: vi.fn() },
  setToken: vi.fn(),
}))

import { LoginStore } from './login.svelte'
import { api, setToken } from './api'
import { router } from './router.svelte'
import { ROUTES } from './routes'

const mockEvent = { preventDefault: vi.fn() } as unknown as SubmitEvent

beforeEach(() => {
  vi.clearAllMocks()
  vi.spyOn(router, 'navigate').mockImplementation(() => {})
})

describe('LoginStore', () => {
  it('starts with empty fields and no error', () => {
    const store = new LoginStore()
    expect(store.username).toBe('')
    expect(store.password).toBe('')
    expect(store.error).toBe('')
    expect(store.loading).toBe(false)
  })

  it('calls api.login, setToken and navigates home on success', async () => {
    vi.mocked(api.login).mockResolvedValue({
      token: 'tok',
      username: 'alice',
      user_id: '1',
      role: 'admin',
      allow_explicit: false,
    })
    const store = new LoginStore()
    store.username = 'alice'
    store.password = 'pw'
    await store.submit(mockEvent)

    expect(api.login).toHaveBeenCalledWith('alice', 'pw')
    expect(setToken).toHaveBeenCalledWith('tok', 'alice', 'admin', false)
    expect(router.navigate).toHaveBeenCalledWith(ROUTES.home, { replace: true })
    expect(store.loading).toBe(false)
  })

  it('sets error on failed login', async () => {
    vi.mocked(api.login).mockRejectedValue(new Error('401'))
    const store = new LoginStore()
    store.username = 'bad'
    store.password = 'bad'
    await store.submit(mockEvent)

    expect(store.error).toMatch(/Invalid/)
    expect(store.loading).toBe(false)
  })
})
