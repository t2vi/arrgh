<script lang="ts">
  import { LoaderCircle } from '@lucide/svelte'
  import { api } from '../../lib/api'
  import type { AppSettings } from '../../lib/types'
  import Button from '../../lib/components/ui/Button.svelte'
  import Input from '../../lib/components/ui/Input.svelte'
  import SettingRow from '../../lib/components/SettingRow.svelte'
  import NumberStepper from '../../lib/components/NumberStepper.svelte'
  import Toggle from '../../lib/components/Toggle.svelte'
  import SegmentedControl from '../../lib/components/SegmentedControl.svelte'

  let { onDone }: { onDone: () => void } = $props()

  const DEFAULTS: AppSettings = {
    download_workers: 2,
    index_interval_hours: 6,
    auto_download: false,
    reader_mode: 'paged',
    download_dir: './downloads',
    trending_per_source: 5,
    check_for_updates: false,
  }

  let workers = $state(DEFAULTS.download_workers)
  let hours = $state(DEFAULTS.index_interval_hours)
  let autoDownload = $state(DEFAULTS.auto_download)
  let readerMode = $state<AppSettings['reader_mode']>(DEFAULTS.reader_mode)
  let downloadDir = $state(DEFAULTS.download_dir)
  let loading = $state(false)

  async function save() {
    loading = true
    try {
      await api.saveSettings({
        download_workers: workers,
        index_interval_hours: hours,
        auto_download: autoDownload,
        reader_mode: readerMode,
        download_dir: downloadDir,
      })
    } finally {
      loading = false
      onDone()
    }
  }
</script>

<div class="space-y-5">
  <div class="space-y-1.5">
    <p class="text-sm font-medium">Library path</p>
    <p class="text-xs text-muted-foreground">Where manga files are stored on the server.</p>
    <Input bind:value={downloadDir} placeholder="./downloads" class="font-mono text-xs" />
  </div>
  <SettingRow label="Download workers" hint="Concurrent chapter downloads (1–10)">
    <NumberStepper value={workers} min={1} max={10} onChange={(n) => (workers = n)} />
  </SettingRow>
  <SettingRow label="Sync interval" hint="Hours between library sync checks (1–24)">
    <NumberStepper value={hours} min={1} max={24} onChange={(n) => (hours = n)} />
  </SettingRow>
  <SettingRow label="Auto-download new chapters" hint="Queue downloads when new chapters appear">
    <Toggle value={autoDownload} onChange={(v) => (autoDownload = v)} label="Auto-download new chapters" />
  </SettingRow>
  <SettingRow label="Default reader mode" hint="Can be overridden per manga">
    <SegmentedControl
      value={readerMode}
      options={[{ value: 'paged', label: 'Paged' }, { value: 'scroll', label: 'Scroll' }]}
      onChange={(v) => (readerMode = v as AppSettings['reader_mode'])}
    />
  </SettingRow>
  <div class="pt-2 space-y-2">
    <Button class="w-full" onclick={save} disabled={loading}>
      {#if loading}<LoaderCircle class="w-4 h-4 animate-spin mr-2" />{/if}
      Save &amp; go to library
    </Button>
    <button
      type="button"
      class="w-full text-xs text-muted-foreground hover:text-foreground transition-colors py-1"
      onclick={onDone}
    >
      Skip, use defaults
    </button>
  </div>
</div>
