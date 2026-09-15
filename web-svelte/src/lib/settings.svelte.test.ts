import { beforeEach, describe, expect, it, vi } from 'vitest'

vi.mock('./api', () => ({
  api: { getSettings: vi.fn(), saveSettings: vi.fn() },
  clearToken: vi.fn(),
  isAdmin: vi.fn(),
}))

import { SettingsStore } from './settings.svelte'
import { api, clearToken, isAdmin } from './api'
import { router } from './router.svelte'
import { ROUTES } from './routes'

const mockSettings = {
  download_workers: 3, index_interval_hours: 6, auto_download: true,
  reader_mode: 'paged' as const, download_dir: './downloads',
  trending_per_source: 10, check_for_updates: true,
}

beforeEach(() => {
  vi.clearAllMocks()
  vi.spyOn(router, 'navigate').mockImplementation(() => {})
  vi.mocked(isAdmin).mockReturnValue(true)
  vi.mocked(api.getSettings).mockResolvedValue(mockSettings)
  vi.mocked(api.saveSettings).mockResolvedValue(mockSettings)
})

describe('SettingsStore', () => {
  it('loads settings on construction', async () => {
    const store = new SettingsStore()
    await vi.waitFor(() => expect(store.isLoading).toBe(false))
    expect(store.settings).toEqual(mockSettings)
  })

  it('defaults to library tab for admins', () => {
    const store = new SettingsStore()
    expect(store.tab).toBe('library')
  })

  it('defaults to account tab for non-admins', () => {
    vi.mocked(isAdmin).mockReturnValue(false)
    const store = new SettingsStore()
    expect(store.tab).toBe('account')
  })

  it('handleSave patches settings', async () => {
    const updated = { ...mockSettings, download_workers: 5, reader_mode: 'scroll' as const }
    vi.mocked(api.saveSettings).mockResolvedValue(updated)
    const store = new SettingsStore()
    await vi.waitFor(() => expect(store.isLoading).toBe(false))
    await store.handleSave({ download_workers: 5 })
    expect(api.saveSettings).toHaveBeenCalledWith({ download_workers: 5 })
    expect(store.settings).toEqual(updated)
  })

  it('logout calls clearToken and navigates to login', () => {
    const store = new SettingsStore()
    store.logout()
    expect(clearToken).toHaveBeenCalled()
    expect(router.navigate).toHaveBeenCalledWith(ROUTES.login, { replace: true })
  })
})
