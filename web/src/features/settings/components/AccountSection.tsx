import { useState } from 'react'
import { Loader2, Check } from 'lucide-react'
import { api } from '@/api'
import { Button } from '@/components/ui/button'
import { Input } from '@/components/ui/input'

export function ChangePasswordSection() {
  const [password, setPassword] = useState('')
  const [confirm, setConfirm] = useState('')
  const [saving, setSaving] = useState(false)
  const [error, setError] = useState('')
  const [saved, setSaved] = useState(false)

  async function handleSubmit(e: React.FormEvent) {
    e.preventDefault()
    setError('')
    if (password.length < 6) { setError('Min 6 characters.'); return }
    if (password !== confirm) { setError('Passwords do not match.'); return }
    setSaving(true)
    try {
      await api.changePassword(password)
      setPassword('')
      setConfirm('')
      setSaved(true)
      setTimeout(() => setSaved(false), 2000)
    } catch {
      setError('Failed to change password.')
    } finally {
      setSaving(false)
    }
  }

  return (
    <section className="space-y-3">
      <h2 className="text-xs font-semibold uppercase tracking-widest text-muted-foreground">Change Password</h2>
      <form onSubmit={handleSubmit} className="space-y-2 max-w-xs">
        <Input
          type="password"
          placeholder="New password"
          value={password}
          onChange={(e) => setPassword(e.target.value)}
          autoComplete="new-password"
          required
        />
        <Input
          type="password"
          placeholder="Confirm password"
          value={confirm}
          onChange={(e) => setConfirm(e.target.value)}
          autoComplete="new-password"
          required
        />
        {error && <p className="text-xs text-destructive">{error}</p>}
        <Button type="submit" size="sm" disabled={saving} className="gap-1.5">
          {saving ? <Loader2 className="w-3.5 h-3.5 animate-spin" /> : saved ? <Check className="w-3.5 h-3.5" /> : null}
          {saved ? 'Saved' : 'Update'}
        </Button>
      </form>
    </section>
  )
}

export function ClientSection() {
  const [url, setUrl] = useState(localStorage.getItem('serverUrl') ?? '')
  const [saved, setSaved] = useState(false)

  function save() {
    localStorage.setItem('serverUrl', url.trim().replace(/\/$/, ''))
    setSaved(true)
    setTimeout(() => setSaved(false), 2000)
  }

  return (
    <section className="space-y-3">
      <h2 className="text-xs font-semibold uppercase tracking-widest text-muted-foreground">Server URL</h2>
      <p className="text-xs text-muted-foreground">Leave blank when the UI is served directly from *ARRgh.</p>
      <Input
        value={url}
        onChange={(e) => setUrl(e.target.value)}
        placeholder="http://192.168.1.x:8080"
        onKeyDown={(e) => e.key === 'Enter' && save()}
        className="max-w-xs"
      />
      <Button onClick={save} size="sm" className="gap-1.5">
        {saved && <Check className="w-3.5 h-3.5" />}
        {saved ? 'Saved' : 'Save'}
      </Button>
    </section>
  )
}
