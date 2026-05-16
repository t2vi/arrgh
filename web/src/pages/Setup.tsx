import { useState } from 'react'
import { useNavigate } from 'react-router-dom'
import { Loader2 } from 'lucide-react'
import { api, setToken } from '@/api'
import { ROUTES } from '@/lib/routes'
import type { AppSettings } from '@/types'
import { Button } from '@/components/ui/button'
import { Input } from '@/components/ui/input'
import { cn } from '@/lib/utils'

// ── Step 1 — Account ──────────────────────────────────────────────────────────

function StepAccount({ onDone }: { onDone: () => void }) {
  const [username, setUsername] = useState('')
  const [password, setPassword] = useState('')
  const [confirm, setConfirm] = useState('')
  const [error, setError] = useState('')
  const [loading, setLoading] = useState(false)

  async function handleSubmit(e: React.FormEvent) {
    e.preventDefault()
    setError('')
    if (password.length < 6) { setError('Password must be at least 6 characters.'); return }
    if (password !== confirm) { setError('Passwords do not match.'); return }
    setLoading(true)
    try {
      const res = await api.register(username.trim(), password)
      setToken(res.token, res.username)
      onDone()
    } catch (err: unknown) {
      const msg = err instanceof Error ? err.message : ''
      if (msg.includes('409') || msg.includes('Conflict')) setError('Username already taken.')
      else if (msg.includes('403')) setError('Setup already complete. Please sign in.')
      else setError('Setup failed. Please try again.')
    } finally {
      setLoading(false)
    }
  }

  return (
    <form onSubmit={handleSubmit} className="space-y-4">
      <div className="space-y-2">
        <label className="text-sm font-medium">Username</label>
        <Input type="text" autoComplete="username" value={username}
          onChange={(e) => setUsername(e.target.value)} placeholder="username" required autoFocus />
      </div>
      <div className="space-y-2">
        <label className="text-sm font-medium">Password</label>
        <Input type="password" autoComplete="new-password" value={password}
          onChange={(e) => setPassword(e.target.value)} placeholder="min 6 characters" required />
      </div>
      <div className="space-y-2">
        <label className="text-sm font-medium">Confirm Password</label>
        <Input type="password" autoComplete="new-password" value={confirm}
          onChange={(e) => setConfirm(e.target.value)} placeholder="repeat password" required />
      </div>
      {error && <p className="text-sm text-destructive">{error}</p>}
      <Button type="submit" className="w-full" disabled={loading}>
        {loading && <Loader2 className="w-4 h-4 animate-spin mr-2" />}
        Create Account
      </Button>
    </form>
  )
}

// ── Step 2 — Settings ─────────────────────────────────────────────────────────

const DEFAULTS: AppSettings = {
  download_workers: 2,
  index_interval_hours: 6,
  auto_download: false,
  reader_mode: 'paged',
}

function StepSettings({ onDone }: { onDone: () => void }) {
  const [workers, setWorkers] = useState(DEFAULTS.download_workers)
  const [hours, setHours] = useState(DEFAULTS.index_interval_hours)
  const [autoDownload, setAutoDownload] = useState(DEFAULTS.auto_download)
  const [readerMode, setReaderMode] = useState<AppSettings['reader_mode']>(DEFAULTS.reader_mode)
  const [loading, setLoading] = useState(false)

  async function save() {
    setLoading(true)
    try {
      await api.saveSettings({ download_workers: workers, index_interval_hours: hours, auto_download: autoDownload, reader_mode: readerMode })
    } finally {
      setLoading(false)
      onDone()
    }
  }

  return (
    <div className="space-y-5">
      <SettingRow label="Download workers" hint="Concurrent chapter downloads (1–10)">
        <NumberStepper value={workers} min={1} max={10} onChange={setWorkers} />
      </SettingRow>
      <SettingRow label="Sync interval" hint="Hours between library sync checks (1–24)">
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
      <div className="pt-2 space-y-2">
        <Button className="w-full" onClick={save} disabled={loading}>
          {loading && <Loader2 className="w-4 h-4 animate-spin mr-2" />}
          Save &amp; go to library
        </Button>
        <button type="button"
          className="w-full text-xs text-muted-foreground hover:text-foreground transition-colors py-1"
          onClick={onDone}>
          Skip, use defaults
        </button>
      </div>
    </div>
  )
}

// ── Primitives ────────────────────────────────────────────────────────────────

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

// ── Root ──────────────────────────────────────────────────────────────────────

export default function Setup() {
  const navigate = useNavigate()
  const [step, setStep] = useState<1 | 2>(1)

  return (
    <div className="min-h-screen bg-background flex items-center justify-center p-4">
      <div className="w-full max-w-sm space-y-8">
        <div className="text-center space-y-2">
          <h1 className="text-4xl font-black tracking-tight text-primary">*ARRgh</h1>
          {step === 1 ? (
            <>
              <p className="text-sm font-semibold">Welcome — create your account</p>
              <p className="text-xs text-muted-foreground">One-time setup for your self-hosted library.</p>
            </>
          ) : (
            <>
              <p className="text-sm font-semibold">Configure your library</p>
              <p className="text-xs text-muted-foreground">These can be changed later in Settings.</p>
            </>
          )}
        </div>

        <div className="flex items-center justify-center gap-2">
          {([1, 2] as const).map((s) => (
            <div key={s} className={cn('w-2 h-2 rounded-full transition-colors', step >= s ? 'bg-primary' : 'bg-muted')} />
          ))}
        </div>

        {step === 1
          ? <StepAccount onDone={() => setStep(2)} />
          : <StepSettings onDone={() => navigate(ROUTES.home, { replace: true })} />
        }
      </div>
    </div>
  )
}
