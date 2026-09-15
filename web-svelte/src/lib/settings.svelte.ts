import { api, clearToken, isAdmin } from './api'
import { router } from './router.svelte'
import { ROUTES } from './routes'
import type { AppSettings } from './types'

export type Tab = 'library' | 'users' | 'sources' | 'account' | 'logs'

export class SettingsStore {
  admin = isAdmin()
  tab = $state<Tab>(this.admin ? 'library' : 'account')
  settings = $state<AppSettings | undefined>(undefined)
  isLoading = $state(true)
  saving = $state(false)

  constructor() {
    api.getSettings()
      .then((s) => (this.settings = s))
      .catch(() => {})
      .finally(() => (this.isLoading = false))
  }

  setTab = (t: Tab) => {
    this.tab = t
  }

  handleSave = async (patch: Partial<AppSettings>) => {
    this.saving = true
    try {
      this.settings = await api.saveSettings(patch)
    } catch {
      // ignore
    } finally {
      this.saving = false
    }
  }

  logout = () => {
    clearToken()
    router.navigate(ROUTES.login, { replace: true })
  }
}
