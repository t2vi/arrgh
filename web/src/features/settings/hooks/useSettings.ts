import { useState, useEffect } from 'react'
import { useNavigate } from 'react-router-dom'
import { api, clearToken, isAdmin } from '@/api'
import { ROUTES } from '@/lib/routes'
import type { AppSettings } from '@/types'

export type Tab = 'library' | 'users' | 'sources' | 'account'

export interface SettingsHandle {
  tab: Tab
  setTab: (t: Tab) => void
  admin: boolean
  settings: AppSettings | undefined
  isLoading: boolean
  saving: boolean
  handleSave: (patch: Partial<AppSettings>) => void
  logout: () => void
}

export function useSettings(): SettingsHandle {
  const navigate = useNavigate()
  const admin = isAdmin()
  const [tab, setTab] = useState<Tab>(admin ? 'library' : 'account')
  const [settings, setSettings] = useState<AppSettings | undefined>()
  const [isLoading, setIsLoading] = useState(true)
  const [saving, setSaving] = useState(false)

  useEffect(() => {
    api.getSettings()
      .then(setSettings)
      .catch(() => {})
      .finally(() => setIsLoading(false))
  }, [])

  async function handleSave(patch: Partial<AppSettings>) {
    setSaving(true)
    try {
      const updated = await api.saveSettings(patch)
      setSettings(updated)
    } catch {
      // ignore
    } finally {
      setSaving(false)
    }
  }

  function logout() {
    clearToken()
    navigate(ROUTES.login, { replace: true })
  }

  return { tab, setTab, admin, settings, isLoading, saving, handleSave, logout }
}
