import { useState } from 'react'
import { api } from '@/api'
import { cn } from '@/lib/utils'
import { Skeleton } from '@/components/ui/skeleton'

// ——— Shared primitives ———

export function SectionHeading({ children }: { children: React.ReactNode }) {
  return (
    <h2 className="text-xs font-semibold uppercase tracking-wider text-muted-foreground flex items-center gap-2">
      <span className="w-0.5 h-3.5 bg-primary rounded-full inline-block shrink-0" />
      {children}
    </h2>
  )
}

export function SegmentControl({
  options,
  active,
  onSelect,
}: {
  options: string[]
  active: number
  onSelect: (i: number) => void
}) {
  return (
    <div className="flex rounded-lg bg-muted p-0.5 gap-0.5">
      {options.map((label, i) => (
        <button
          key={label}
          onClick={() => onSelect(i)}
          className={cn(
            'flex-1 text-[11px] font-medium py-1.5 rounded-md transition-colors',
            active === i
              ? 'bg-card text-foreground shadow-sm'
              : 'text-muted-foreground hover:text-foreground',
          )}
        >
          {label}
        </button>
      ))}
    </div>
  )
}

function SettingSegmentCard({
  label,
  description,
  children,
}: {
  label: string
  description: string
  children: React.ReactNode
}) {
  return (
    <div className="rounded-lg bg-card border border-border p-4 space-y-3">
      <p className="text-[10px] font-semibold uppercase tracking-wider text-muted-foreground">{label}</p>
      {children}
      <p className="text-[10px] text-muted-foreground leading-relaxed">{description}</p>
    </div>
  )
}

// ——— Setting cards ———

export function ReaderModeCard({ mangaId, value }: { mangaId: string; value: string | null }) {
  const [current, setCurrent] = useState(value)
  const options: { label: string; v: string | null }[] = [
    { label: 'Global', v: null },
    { label: 'Paged',  v: 'paged' },
    { label: 'Scroll', v: 'scroll' },
  ]

  async function handleSelect(i: number) {
    const v = options[i].v
    setCurrent(v)
    await api.setMangaReaderMode(mangaId, v).catch(() => {})
  }

  return (
    <SettingSegmentCard
      label="Reader Mode"
      description={
        current == null
          ? 'Uses the global reader mode setting.'
          : current === 'paged'
          ? 'One page at a time, tap/click to advance.'
          : 'All pages in a continuous vertical scroll.'
      }
    >
      <SegmentControl
        options={options.map((o) => o.label)}
        active={options.findIndex((o) => o.v === current)}
        onSelect={handleSelect}
      />
    </SettingSegmentCard>
  )
}

export function AutoDownloadCard({ mangaId, value }: { mangaId: string; value: boolean | null }) {
  const [current, setCurrent] = useState(value)
  const options: { label: string; v: boolean | null }[] = [
    { label: 'Global', v: null },
    { label: 'Always', v: true },
    { label: 'Never',  v: false },
  ]

  async function handleSelect(i: number) {
    const v = options[i].v
    setCurrent(v)
    await api.setMangaAutoDownload(mangaId, v).catch(() => {})
  }

  return (
    <SettingSegmentCard
      label="Auto-download"
      description={
        current === null
          ? 'Follows the global auto-download setting.'
          : current
          ? 'New chapters download automatically.'
          : 'New chapters are never auto-downloaded.'
      }
    >
      <SegmentControl
        options={options.map((o) => o.label)}
        active={options.findIndex((o) => o.v === current)}
        onSelect={handleSelect}
      />
    </SettingSegmentCard>
  )
}

export function ExplicitCard({ mangaId, value }: { mangaId: string; value: boolean }) {
  const [current, setCurrent] = useState(value)

  async function toggle() {
    const next = !current
    setCurrent(next)
    await api.setMangaExplicit(mangaId, next).catch(() => setCurrent(current))
  }

  return (
    <div className="rounded-lg bg-card border border-border p-4 space-y-2">
      <div className="flex items-center justify-between gap-3">
        <p className="text-[10px] font-semibold uppercase tracking-wider text-muted-foreground">
          Explicit Content
        </p>
        <button
          type="button"
          onClick={toggle}
          className={`relative w-10 h-[22px] rounded-full transition-colors ${current ? 'bg-orange-500' : 'bg-muted'}`}
        >
          <span className={`absolute top-[3px] w-4 h-4 rounded-full bg-white shadow transition-transform ${current ? 'translate-x-5' : 'translate-x-[3px]'}`} />
        </button>
      </div>
      <p className="text-xs text-muted-foreground">
        {current ? 'Marked 18+ — only users with explicit access can see this.' : 'Not marked explicit — visible to all users.'}
      </p>
    </div>
  )
}

export function DownloadDirCard({ mangaId, value }: { mangaId: string; value: string | null }) {
  const [draft, setDraft] = useState(value ?? '')
  const [saving, setSaving] = useState(false)

  async function handleSave(v: string | null) {
    setSaving(true)
    await api.setMangaDownloadDir(mangaId, v).catch(() => {})
    setSaving(false)
  }

  const isDirty = draft !== (value ?? '')
  const isDefault = !value

  return (
    <div className="rounded-lg bg-card border border-border p-4 space-y-3">
      <p className="text-[10px] font-semibold uppercase tracking-wider text-muted-foreground">
        Download Path
      </p>
      <div className="flex gap-2">
        <input
          type="text"
          value={draft}
          onChange={(e) => setDraft(e.target.value)}
          onKeyDown={(e) => { if (e.key === 'Enter') handleSave(draft.trim() || null) }}
          placeholder="/path/to/manga"
          className="flex-1 bg-muted border border-transparent focus:border-ring rounded-md px-3 py-1.5 text-xs outline-none transition-colors"
        />
        {isDirty && (
          <button
            onClick={() => handleSave(draft.trim() || null)}
            disabled={saving}
            className="px-3 py-1.5 rounded-md bg-primary text-primary-foreground text-xs font-semibold hover:opacity-90 transition-opacity"
          >
            Save
          </button>
        )}
        {!isDirty && !isDefault && (
          <button
            onClick={() => { setDraft(''); handleSave(null) }}
            className="px-3 py-1.5 rounded-md bg-muted border border-border text-xs text-muted-foreground hover:text-foreground transition-colors"
          >
            Reset
          </button>
        )}
      </div>
      <p className="text-[10px] text-muted-foreground leading-relaxed">
        {isDefault
          ? 'Uses default: _downloads/{title}/ inside your manga dir.'
          : `Chapters save to ${value}`}
      </p>
    </div>
  )
}

export function CoverImg({ coverUrl, mangaId }: { coverUrl: string | null; mangaId: string }) {
  const [failed, setFailed] = useState(false)
  const src = !failed && coverUrl?.startsWith('http') ? coverUrl : api.coverUrl(mangaId)
  if (failed) {
    return (
      <div className="w-36 shrink-0 rounded-xl aspect-[2/3] bg-muted/60 flex items-center justify-center text-4xl shadow-2xl ring-1 ring-white/10">
        📖
      </div>
    )
  }
  return (
    <img
      src={src}
      alt=""
      className="w-36 shrink-0 rounded-xl aspect-[2/3] object-cover bg-muted shadow-2xl ring-1 ring-white/10"
      onError={() => setFailed(true)}
    />
  )
}

export function ChapterListSkeleton() {
  return (
    <div className="space-y-1.5">
      {Array.from({ length: 5 }).map((_, i) => (
        <div key={i} className="flex items-center gap-3 px-3 py-2.5 rounded-lg border border-border bg-card">
          <Skeleton className="w-11 h-7 rounded-md shrink-0" />
          <div className="flex-1 space-y-1.5">
            <Skeleton className="h-4 w-1/2" />
            <Skeleton className="h-3 w-1/3" />
          </div>
          <Skeleton className="w-7 h-7 rounded-md shrink-0" />
        </div>
      ))}
    </div>
  )
}
