<script lang="ts">
  import { LoaderCircle, Check } from '@lucide/svelte'
  import type { AppSettings } from '../../lib/types'
  import Button from '../../lib/components/ui/Button.svelte'
  import Input from '../../lib/components/ui/Input.svelte'
  import SettingRow from '../../lib/components/SettingRow.svelte'
  import NumberStepper from '../../lib/components/NumberStepper.svelte'
  import Toggle from '../../lib/components/Toggle.svelte'
  import SegmentedControl from '../../lib/components/SegmentedControl.svelte'

  let {
    settings, saving, onSave,
  }: {
    settings: AppSettings
    saving: boolean
    onSave: (patch: Partial<AppSettings>) => void
  } = $props()

  // svelte-ignore state_referenced_locally
  let workers = $state(settings.download_workers)
  // svelte-ignore state_referenced_locally
  let hours = $state(settings.index_interval_hours)
  // svelte-ignore state_referenced_locally
  let autoDownload = $state(settings.auto_download)
  // svelte-ignore state_referenced_locally
  let readerMode = $state<AppSettings['reader_mode']>(settings.reader_mode)
  // svelte-ignore state_referenced_locally
  let downloadDir = $state(settings.download_dir)
  // svelte-ignore state_referenced_locally
  let trendingPerSource = $state(settings.trending_per_source)
  // svelte-ignore state_referenced_locally
  let checkForUpdates = $state(settings.check_for_updates)
  let saved = $state(false)

  function handleSave() {
    onSave({
      download_workers: workers,
      index_interval_hours: hours,
      auto_download: autoDownload,
      reader_mode: readerMode,
      download_dir: downloadDir,
      trending_per_source: trendingPerSource,
      check_for_updates: checkForUpdates,
    })
    saved = true
    setTimeout(() => (saved = false), 2000)
  }
</script>

<section class="space-y-5">
  <h2 class="text-xs font-semibold uppercase tracking-widest text-muted-foreground">Downloads</h2>
  <SettingRow label="Download workers" hint="Concurrent chapter downloads (1–10)">
    <NumberStepper value={workers} min={1} max={10} onChange={(v) => (workers = v)} />
  </SettingRow>
  <SettingRow label="Sync interval (hours)" hint="How often to check for new chapters">
    <NumberStepper value={hours} min={1} max={24} onChange={(v) => (hours = v)} />
  </SettingRow>
  <SettingRow label="Auto-download new chapters" hint="Queue downloads when new chapters appear">
    <Toggle value={autoDownload} onChange={(v) => (autoDownload = v)} />
  </SettingRow>

  <h2 class="text-xs font-semibold uppercase tracking-widest text-muted-foreground pt-2">Storage</h2>
  <div class="space-y-1.5">
    <p class="text-sm font-medium">Library path</p>
    <p class="text-xs text-muted-foreground">Absolute or relative path. Restart required to apply.</p>
    <Input
      bind:value={downloadDir}
      placeholder="./downloads"
      class="max-w-xs font-mono text-xs"
    />
  </div>

  <h2 class="text-xs font-semibold uppercase tracking-widest text-muted-foreground pt-2">Discover</h2>
  <SettingRow label="Trending titles per source" hint="How many results to show per source in Trending (1–20)">
    <NumberStepper value={trendingPerSource} min={1} max={20} onChange={(v) => (trendingPerSource = v)} />
  </SettingRow>

  <h2 class="text-xs font-semibold uppercase tracking-widest text-muted-foreground pt-2">Reader</h2>
  <SettingRow label="Default reader mode" hint="Can be overridden per manga">
    <SegmentedControl
      value={readerMode}
      options={[{ value: 'paged', label: 'Paged' }, { value: 'scroll', label: 'Scroll' }]}
      onChange={(v) => (readerMode = v as AppSettings['reader_mode'])}
    />
  </SettingRow>

  <h2 class="text-xs font-semibold uppercase tracking-widest text-muted-foreground pt-2">Updates</h2>
  <SettingRow label="Check for updates" hint="Polls GitHub Releases once per hour">
    <Toggle value={checkForUpdates} onChange={(v) => (checkForUpdates = v)} />
  </SettingRow>

  <Button onclick={handleSave} disabled={saving} class="gap-1.5">
    {#if saving}
      <LoaderCircle class="w-3.5 h-3.5 animate-spin" />
    {:else if saved}
      <Check class="w-3.5 h-3.5" />
    {/if}
    {saved ? 'Saved' : 'Save'}
  </Button>
</section>
