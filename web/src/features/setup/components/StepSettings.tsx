import { useState } from 'react'
import { Loader2 } from 'lucide-react'
import { api } from '@/api'
import type { AppSettings } from '@/types'
import { Button } from '@/components/ui/button'
import { Input } from '@/components/ui/input'
import { SettingRow } from '@/components/SettingRow'
import { NumberStepper } from '@/components/NumberStepper'
import { Toggle } from '@/components/Toggle'
import { SegmentedControl } from '@/components/SegmentedControl'

const DEFAULTS: AppSettings = {
  download_workers: 2,
  index_interval_hours: 6,
  auto_download: false,
  reader_mode: 'paged',
  download_dir: './downloads',
  trending_per_source: 5,
}

export function StepSettings({ onDone }: { onDone: () => void }) {
  const [workers, setWorkers] = useState(DEFAULTS.download_workers)
  const [hours, setHours] = useState(DEFAULTS.index_interval_hours)
  const [autoDownload, setAutoDownload] = useState(DEFAULTS.auto_download)
  const [readerMode, setReaderMode] = useState<AppSettings['reader_mode']>(DEFAULTS.reader_mode)
  const [downloadDir, setMangaDir] = useState(DEFAULTS.download_dir)
  const [loading, setLoading] = useState(false)

  async function save() {
    setLoading(true)
    try {
      await api.saveSettings({
        download_workers: workers,
        index_interval_hours: hours,
        auto_download: autoDownload,
        reader_mode: readerMode,
        download_dir: downloadDir,
      })
    } finally {
      setLoading(false)
      onDone()
    }
  }

  return (
    <div className="space-y-5">
      <div className="space-y-1.5">
        <p className="text-sm font-medium">Library path</p>
        <p className="text-xs text-muted-foreground">Where manga files are stored on the server.</p>
        <Input
          value={downloadDir}
          onChange={(e) => setMangaDir(e.target.value)}
          placeholder="./downloads"
          className="font-mono text-xs"
        />
      </div>
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
        <button
          type="button"
          className="w-full text-xs text-muted-foreground hover:text-foreground transition-colors py-1"
          onClick={onDone}
        >
          Skip, use defaults
        </button>
      </div>
    </div>
  )
}
