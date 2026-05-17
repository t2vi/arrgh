import { useState, useEffect, useCallback } from 'react'
import { ChevronRight, Loader2, Check, Trash2, ShieldCheck, Eye, Plus, Globe, KeyRound } from 'lucide-react'
import { useNavigate } from 'react-router-dom'
import { api, clearToken, isAdmin, type UserListItem, type SourceRow } from '@/api'
import { ROUTES } from '@/lib/routes'
import type { AppSettings } from '@/types'
import { Button } from '@/components/ui/button'
import { Input } from '@/components/ui/input'
import { cn } from '@/lib/utils'

type Tab = 'library' | 'users' | 'sources' | 'account'

const ADMIN_TABS: { id: Tab; label: string }[] = [
  { id: 'library', label: 'Library' },
  { id: 'users',   label: 'Users'   },
  { id: 'sources', label: 'Sources' },
  { id: 'account', label: 'Account' },
]

export default function Settings() {
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

  if (isLoading) {
    return (
      <div className="flex-1 flex items-center justify-center">
        <Loader2 className="w-5 h-5 animate-spin text-muted-foreground" />
      </div>
    )
  }

  return (
    <div className="flex flex-col h-full">
      <header className="flex items-center gap-3 px-4 py-3 border-b border-border bg-card shrink-0">
        <Button variant="ghost" size="icon" onClick={() => navigate(-1)}>
          <ChevronRight className="w-4 h-4 rotate-180" />
        </Button>
        <h1 className="text-base font-semibold">Settings</h1>

        {admin && (
          <div className="ml-4 flex rounded-lg bg-muted p-0.5 gap-0.5">
            {ADMIN_TABS.map((t) => (
              <button
                key={t.id}
                type="button"
                onClick={() => setTab(t.id)}
                className={cn(
                  'px-3 py-1 rounded-md text-xs font-semibold transition-colors',
                  tab === t.id
                    ? 'bg-card text-foreground shadow-sm'
                    : 'text-muted-foreground hover:text-foreground',
                )}
              >
                {t.label}
              </button>
            ))}
          </div>
        )}
      </header>

      <div className="flex-1 overflow-auto p-6 max-w-lg">
        {tab === 'library' && admin && (
          <div className="space-y-8">
            {settings
              ? <ServerSettingsSection settings={settings} saving={saving} onSave={handleSave} />
              : <p className="text-sm text-muted-foreground">Failed to load settings.</p>
            }
          </div>
        )}

        {tab === 'users' && admin && (
          <UsersSection />
        )}

        {tab === 'sources' && admin && (
          <SourcesSection />
        )}

        {tab === 'account' && (
          <div className="space-y-8">
            <ChangePasswordSection />
            <ClientSection />
            <section>
              <h2 className="text-xs font-semibold uppercase tracking-widest text-muted-foreground mb-3">Session</h2>
              <Button variant="outline" onClick={logout} className="text-destructive border-destructive/40 hover:bg-destructive/10">
                Sign out
              </Button>
            </section>
          </div>
        )}
      </div>
    </div>
  )
}

// ── Library (admin) ───────────────────────────────────────────────────────────

function ServerSettingsSection({
  settings, saving, onSave,
}: {
  settings: AppSettings
  saving: boolean
  onSave: (patch: Partial<AppSettings>) => void
}) {
  const [workers, setWorkers] = useState(settings.download_workers)
  const [hours, setHours] = useState(settings.index_interval_hours)
  const [autoDownload, setAutoDownload] = useState(settings.auto_download)
  const [readerMode, setReaderMode] = useState<AppSettings['reader_mode']>(settings.reader_mode)
  const [mangaDir, setMangaDir] = useState(settings.manga_dir)
  const [saved, setSaved] = useState(false)

  function handleSave() {
    onSave({ download_workers: workers, index_interval_hours: hours, auto_download: autoDownload, reader_mode: readerMode, manga_dir: mangaDir })
    setSaved(true)
    setTimeout(() => setSaved(false), 2000)
  }

  return (
    <section className="space-y-5">
      <h2 className="text-xs font-semibold uppercase tracking-widest text-muted-foreground">Downloads</h2>
      <SettingRow label="Download workers" hint="Concurrent chapter downloads (1–10)">
        <NumberStepper value={workers} min={1} max={10} onChange={setWorkers} />
      </SettingRow>
      <SettingRow label="Sync interval (hours)" hint="How often to check for new chapters">
        <NumberStepper value={hours} min={1} max={24} onChange={setHours} />
      </SettingRow>
      <SettingRow label="Auto-download new chapters" hint="Queue downloads when new chapters appear">
        <Toggle value={autoDownload} onChange={setAutoDownload} />
      </SettingRow>

      <h2 className="text-xs font-semibold uppercase tracking-widest text-muted-foreground pt-2">Storage</h2>
      <div className="space-y-1.5">
        <p className="text-sm font-medium">Library path</p>
        <p className="text-xs text-muted-foreground">Absolute or relative path. Restart required to apply.</p>
        <Input
          value={mangaDir}
          onChange={(e) => setMangaDir(e.target.value)}
          placeholder="./manga"
          className="max-w-xs font-mono text-xs"
        />
      </div>

      <h2 className="text-xs font-semibold uppercase tracking-widest text-muted-foreground pt-2">Reader</h2>
      <SettingRow label="Default reader mode" hint="Can be overridden per manga">
        <SegmentedControl
          value={readerMode}
          options={[{ value: 'paged', label: 'Paged' }, { value: 'scroll', label: 'Scroll' }]}
          onChange={(v) => setReaderMode(v as AppSettings['reader_mode'])}
        />
      </SettingRow>

      <Button onClick={handleSave} disabled={saving} className="gap-1.5">
        {saving ? <Loader2 className="w-3.5 h-3.5 animate-spin" /> : saved ? <Check className="w-3.5 h-3.5" /> : null}
        {saved ? 'Saved' : 'Save'}
      </Button>
    </section>
  )
}

// ── Users (admin) ─────────────────────────────────────────────────────────────

function UsersSection() {
  const [users, setUsers] = useState<UserListItem[]>([])
  const [loading, setLoading] = useState(true)
  const [newUsername, setNewUsername] = useState('')
  const [newPassword, setNewPassword] = useState('')
  const [createError, setCreateError] = useState('')
  const [creating, setCreating] = useState(false)

  const reload = useCallback(() => {
    setLoading(true)
    api.listUsers()
      .then(setUsers)
      .catch(() => {})
      .finally(() => setLoading(false))
  }, [])

  useEffect(() => { reload() }, [reload])

  async function handlePatch(id: string, patch: Parameters<typeof api.patchUser>[1]) {
    await api.patchUser(id, patch)
    reload()
  }

  async function handleDelete(id: string) {
    await api.deleteUser(id)
    reload()
  }

  async function handleCreate(e: React.FormEvent) {
    e.preventDefault()
    setCreateError('')
    if (newPassword.length < 6) { setCreateError('Password must be at least 6 characters.'); return }
    setCreating(true)
    try {
      await api.createUser(newUsername.trim(), newPassword)
      setNewUsername('')
      setNewPassword('')
      reload()
    } catch (err: unknown) {
      const msg = err instanceof Error ? err.message : ''
      if (msg.includes('409')) setCreateError('Username already taken.')
      else setCreateError('Failed to create user.')
    } finally {
      setCreating(false)
    }
  }

  return (
    <section className="space-y-4">
      {loading ? (
        <Loader2 className="w-4 h-4 animate-spin text-muted-foreground" />
      ) : (
        <div className="space-y-1.5">
          {users.map((u: UserListItem) => (
            <UserRow
              key={u.id}
              user={u}
              onPatch={(patch) => handlePatch(u.id, patch)}
              onDelete={() => handleDelete(u.id)}
            />
          ))}
        </div>
      )}

      <form onSubmit={handleCreate} className="space-y-2 pt-2 border-t border-border">
        <p className="text-xs font-medium text-muted-foreground">Add member</p>
        <div className="flex gap-2">
          <Input
            placeholder="Username"
            value={newUsername}
            onChange={(e) => setNewUsername(e.target.value)}
            required
            className="flex-1"
          />
          <Input
            type="password"
            placeholder="Password"
            value={newPassword}
            onChange={(e) => setNewPassword(e.target.value)}
            required
            className="flex-1"
          />
          <Button type="submit" size="sm" disabled={creating}>
            {creating ? <Loader2 className="w-3.5 h-3.5 animate-spin" /> : 'Add'}
          </Button>
        </div>
        {createError && <p className="text-xs text-destructive">{createError}</p>}
      </form>
    </section>
  )
}

function UserRow({ user, onPatch, onDelete }: {
  user: UserListItem
  onPatch: (patch: { role?: string; allow_explicit?: boolean }) => void
  onDelete: () => void
}) {
  const isSelf = localStorage.getItem('arrgh_username') === user.username

  return (
    <div className="flex items-center gap-2 px-3 py-2 rounded-lg bg-muted/50">
      <div className="flex-1 min-w-0">
        <p className="text-sm font-medium truncate">{user.username}</p>
      </div>

      <button
        type="button"
        title={user.role === 'admin' ? 'Admin — click to demote' : 'Member — click to promote'}
        onClick={() => !isSelf && onPatch({ role: user.role === 'admin' ? 'member' : 'admin' })}
        disabled={isSelf}
        className={cn(
          'flex items-center gap-1 px-2 py-0.5 rounded-full text-[10px] font-semibold transition-colors',
          user.role === 'admin'
            ? 'bg-primary/20 text-primary hover:bg-primary/30'
            : 'bg-muted text-muted-foreground hover:bg-accent',
          isSelf && 'opacity-50 cursor-default',
        )}
      >
        <ShieldCheck className="w-3 h-3" />
        {user.role}
      </button>

      <button
        type="button"
        title={user.allow_explicit ? 'Explicit allowed — click to revoke' : 'Explicit blocked — click to allow'}
        onClick={() => onPatch({ allow_explicit: !user.allow_explicit })}
        className={cn(
          'flex items-center gap-1 px-2 py-0.5 rounded-full text-[10px] font-semibold transition-colors',
          user.allow_explicit
            ? 'bg-orange-500/20 text-orange-400 hover:bg-orange-500/30'
            : 'bg-muted text-muted-foreground hover:bg-accent',
        )}
      >
        <Eye className="w-3 h-3" />
        18+
      </button>

      {!isSelf && (
        <button
          type="button"
          title="Delete user"
          onClick={onDelete}
          className="w-6 h-6 flex items-center justify-center rounded-md text-muted-foreground hover:text-destructive hover:bg-destructive/10 transition-colors"
        >
          <Trash2 className="w-3.5 h-3.5" />
        </button>
      )}
    </div>
  )
}

// ── Sources (admin) ───────────────────────────────────────────────────────────

function SourcesSection() {
  const [sources, setSources] = useState<SourceRow[]>([])
  const [loading, setLoading] = useState(true)
  const [url, setUrl] = useState('')
  const [apiKey, setApiKey] = useState('')
  const [adding, setAdding] = useState(false)
  const [addError, setAddError] = useState('')

  const reload = useCallback(() => {
    setLoading(true)
    api.listSources()
      .then(setSources)
      .catch(() => {})
      .finally(() => setLoading(false))
  }, [])

  useEffect(() => { reload() }, [reload])

  async function handleAdd(e: React.FormEvent) {
    e.preventDefault()
    setAddError('')
    setAdding(true)
    try {
      await api.addSource(url.trim(), apiKey.trim() || undefined)
      setUrl('')
      setApiKey('')
      reload()
    } catch (err: unknown) {
      const msg = err instanceof Error ? err.message : ''
      if (msg.includes('502')) setAddError('Could not reach plugin — check the URL.')
      else setAddError('Failed to add source.')
    } finally {
      setAdding(false)
    }
  }

  async function handleToggle(id: string, enabled: boolean) {
    await api.patchSource(id, !enabled)
    reload()
  }

  async function handleDelete(id: string) {
    await api.deleteSource(id)
    reload()
  }

  return (
    <section className="space-y-4">
      <p className="text-xs text-muted-foreground">
        External source plugins extend *ARRgh with additional manga sources. Each plugin is a separate HTTP server implementing the Source Plugin Protocol.
      </p>

      {loading ? (
        <Loader2 className="w-4 h-4 animate-spin text-muted-foreground" />
      ) : sources.length === 0 ? (
        <p className="text-sm text-muted-foreground">No external sources yet.</p>
      ) : (
        <div className="space-y-1.5">
          {sources.map((s) => (
            <SourceRowItem key={s.id} source={s} onToggle={handleToggle} onDelete={handleDelete} />
          ))}
        </div>
      )}

      <form onSubmit={handleAdd} className="space-y-2 pt-2 border-t border-border">
        <p className="text-xs font-medium text-muted-foreground">Add plugin source</p>
        <div className="flex gap-2">
          <div className="relative flex-1">
            <Globe className="absolute left-2.5 top-1/2 -translate-y-1/2 w-3.5 h-3.5 text-muted-foreground pointer-events-none" />
            <input
              className="w-full pl-8 pr-3 py-1.5 rounded-md bg-muted border border-border text-sm placeholder:text-muted-foreground focus:outline-none focus:ring-1 focus:ring-ring"
              placeholder="http://localhost:4000"
              value={url}
              onChange={(e) => setUrl(e.target.value)}
              required
            />
          </div>
          <div className="relative w-40">
            <KeyRound className="absolute left-2.5 top-1/2 -translate-y-1/2 w-3.5 h-3.5 text-muted-foreground pointer-events-none" />
            <input
              className="w-full pl-8 pr-3 py-1.5 rounded-md bg-muted border border-border text-sm placeholder:text-muted-foreground focus:outline-none focus:ring-1 focus:ring-ring"
              placeholder="API key (opt.)"
              value={apiKey}
              onChange={(e) => setApiKey(e.target.value)}
            />
          </div>
          <button
            type="submit"
            disabled={adding}
            className="flex items-center gap-1 px-3 py-1.5 rounded-md bg-primary text-primary-foreground text-sm font-medium hover:bg-primary/90 disabled:opacity-60 transition-colors"
          >
            {adding ? <Loader2 className="w-3.5 h-3.5 animate-spin" /> : <Plus className="w-3.5 h-3.5" />}
            Add
          </button>
        </div>
        {addError && <p className="text-xs text-destructive">{addError}</p>}
      </form>
    </section>
  )
}

function SourceRowItem({ source, onToggle, onDelete }: {
  source: SourceRow
  onToggle: (id: string, enabled: boolean) => void
  onDelete: (id: string) => void
}) {
  return (
    <div className="flex items-center gap-2 px-3 py-2 rounded-lg bg-muted/50">
      <div className="flex-1 min-w-0">
        <div className="flex items-center gap-2">
          <p className="text-sm font-medium truncate">{source.name}</p>
          {source.has_api_key && (
            <KeyRound className="w-3 h-3 text-muted-foreground shrink-0" title="Uses API key" />
          )}
        </div>
        <div className="flex items-center gap-1.5 mt-0.5 flex-wrap">
          <p className="text-xs text-muted-foreground truncate">{source.base_url}</p>
          {source.content_types.map((ct) => (
            <span key={ct} className="px-1.5 py-0 rounded-full bg-card border border-border text-[10px] text-muted-foreground">
              {ct}
            </span>
          ))}
        </div>
      </div>

      <button
        type="button"
        title={source.enabled ? 'Enabled — click to disable' : 'Disabled — click to enable'}
        onClick={() => onToggle(source.id, source.enabled)}
        className={cn(
          'relative w-8 h-[18px] rounded-full transition-colors shrink-0',
          source.enabled ? 'bg-primary' : 'bg-muted border border-border',
        )}
      >
        <span className={cn(
          'absolute top-[2px] w-3.5 h-3.5 rounded-full bg-white shadow transition-transform',
          source.enabled ? 'translate-x-[14px]' : 'translate-x-[2px]',
        )} />
      </button>

      <button
        type="button"
        title="Remove source"
        onClick={() => onDelete(source.id)}
        className="w-6 h-6 flex items-center justify-center rounded-md text-muted-foreground hover:text-destructive hover:bg-destructive/10 transition-colors"
      >
        <Trash2 className="w-3.5 h-3.5" />
      </button>
    </div>
  )
}

// ── Account (all users) ───────────────────────────────────────────────────────

function ChangePasswordSection() {
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

function ClientSection() {
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

// ── Shared primitives ─────────────────────────────────────────────────────────

function SettingRow({ label, hint, children }: { label: string; hint: string; children: React.ReactNode }) {
  return (
    <div className="flex items-center justify-between gap-4">
      <div className="min-w-0">
        <p className="text-sm font-medium">{label}</p>
        <p className="text-xs text-muted-foreground">{hint}</p>
      </div>
      <div className="shrink-0">{children}</div>
    </div>
  )
}

function NumberStepper({ value, min, max, onChange }: { value: number; min: number; max: number; onChange: (n: number) => void }) {
  return (
    <div className="flex items-center gap-2">
      <button type="button"
        className="w-7 h-7 rounded-md bg-muted text-sm flex items-center justify-center hover:bg-accent disabled:opacity-40"
        disabled={value <= min} onClick={() => onChange(value - 1)}>−</button>
      <span className="w-5 text-center text-sm font-semibold tabular-nums">{value}</span>
      <button type="button"
        className="w-7 h-7 rounded-md bg-muted text-sm flex items-center justify-center hover:bg-accent disabled:opacity-40"
        disabled={value >= max} onClick={() => onChange(value + 1)}>+</button>
    </div>
  )
}

function Toggle({ value, onChange }: { value: boolean; onChange: (v: boolean) => void }) {
  return (
    <button type="button" role="switch" aria-checked={value}
      className={cn('relative w-10 h-[22px] rounded-full transition-colors', value ? 'bg-primary' : 'bg-muted')}
      onClick={() => onChange(!value)}>
      <span className={cn('absolute top-[3px] w-4 h-4 rounded-full bg-white shadow transition-transform', value ? 'translate-x-5' : 'translate-x-[3px]')} />
    </button>
  )
}

function SegmentedControl({ value, options, onChange }: {
  value: string; options: { value: string; label: string }[]; onChange: (v: string) => void
}) {
  return (
    <div className="flex rounded-lg bg-muted p-0.5 gap-0.5">
      {options.map((o) => (
        <button key={o.value} type="button"
          className={cn('px-3 py-1 rounded-md text-xs font-semibold transition-colors',
            value === o.value ? 'bg-card text-foreground shadow-sm' : 'text-muted-foreground hover:text-foreground')}
          onClick={() => onChange(o.value)}>
          {o.label}
        </button>
      ))}
    </div>
  )
}
