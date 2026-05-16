import { useState } from 'react'
import { useQuery, useMutation, useQueryClient } from '@tanstack/react-query'
import { ChevronRight, Loader2, Check } from 'lucide-react'
import { useNavigate } from 'react-router-dom'
import { api, clearToken } from '@/api'
import { ROUTES } from '@/lib/routes'
import type { AppSettings } from '@/types'
import { Button } from '@/components/ui/button'
import { Input } from '@/components/ui/input'
import { cn } from '@/lib/utils'

export default function Settings() {
  const navigate = useNavigate()
  const qc = useQueryClient()

  const { data: settings, isLoading } = useQuery({
    queryKey: ['settings'],
    queryFn: () => api.getSettings(),
  })

  const save = useMutation({
    mutationFn: (s: Partial<AppSettings>) => api.saveSettings(s),
    onSuccess: (data) => qc.setQueryData(['settings'], data),
  })

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
      <header className="flex items-center gap-2 px-4 py-3 border-b border-border bg-card shrink-0">
        <Button variant="ghost" size="icon" onClick={() => navigate(-1)}>
          <ChevronRight className="w-4 h-4 rotate-180" />
        </Button>
        <h1 className="text-base font-semibold">Settings</h1>
      </header>

      <div className="flex-1 overflow-auto p-6 space-y-8 max-w-lg">
        {settings && (
          <ServerSettingsSection
            settings={settings}
            saving={save.isPending}
            onSave={(patch) => save.mutate(patch)}
          />
        )}

        <ClientSection />

        <section>
          <h2 className="text-xs font-semibold uppercase tracking-widest text-muted-foreground mb-3">Account</h2>
          <Button variant="outline" onClick={logout} className="text-destructive border-destructive/40 hover:bg-destructive/10">
            Sign out
          </Button>
        </section>
      </div>
    </div>
  )
}

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
  const [saved, setSaved] = useState(false)

  function handleSave() {
    onSave({ download_workers: workers, index_interval_hours: hours, auto_download: autoDownload, reader_mode: readerMode })
    setSaved(true)
    setTimeout(() => setSaved(false), 2000)
  }

  return (
    <section className="space-y-5">
      <h2 className="text-xs font-semibold uppercase tracking-widest text-muted-foreground">Library</h2>
      <SettingRow label="Download workers" hint="Concurrent chapter downloads (1–10)">
        <NumberStepper value={workers} min={1} max={10} onChange={setWorkers} />
      </SettingRow>
      <SettingRow label="Sync interval (hours)" hint="How often to check for new chapters">
        <NumberStepper value={hours} min={1} max={24} onChange={setHours} />
      </SettingRow>
      <SettingRow label="Auto-download new chapters" hint="Queue downloads when new chapters appear">
        <Toggle value={autoDownload} onChange={setAutoDownload} />
      </SettingRow>
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
      <h2 className="text-xs font-semibold uppercase tracking-widest text-muted-foreground">Client</h2>
      <div className="space-y-1.5">
        <p className="text-sm font-medium">Server URL</p>
        <p className="text-xs text-muted-foreground">Leave blank when the UI is served directly from *ARRgh.</p>
      </div>
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
