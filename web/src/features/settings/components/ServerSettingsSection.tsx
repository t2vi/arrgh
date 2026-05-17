import { useState } from 'react'
import { Loader2, Check } from 'lucide-react'
import type { AppSettings } from '@/types'
import { Button } from '@/components/ui/button'
import { Input } from '@/components/ui/input'
import { SettingRow } from '@/components/SettingRow'
import { NumberStepper } from '@/components/NumberStepper'
import { Toggle } from '@/components/Toggle'
import { SegmentedControl } from '@/components/SegmentedControl'

export function ServerSettingsSection({
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
