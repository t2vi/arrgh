<script lang="ts">
  import { LoaderCircle, Check, Trash2, RotateCcw, Upload } from '@lucide/svelte'
  import type { AppSettings } from '../../lib/types'
  import { BackupsStore } from '../../lib/backups.svelte'
  import Button from '../../lib/components/ui/Button.svelte'
  import Input from '../../lib/components/ui/Input.svelte'
  import SettingRow from '../../lib/components/SettingRow.svelte'
  import NumberStepper from '../../lib/components/NumberStepper.svelte'

  let {
    settings, saving, onSave,
  }: {
    settings: AppSettings
    saving: boolean
    onSave: (patch: Partial<AppSettings>) => void
  } = $props()

  // svelte-ignore state_referenced_locally
  let backupDir = $state(settings.backup_dir)
  // svelte-ignore state_referenced_locally
  let intervalHours = $state(settings.backup_interval_hours)
  let saved = $state(false)

  function handleSave() {
    onSave({ backup_dir: backupDir, backup_interval_hours: intervalHours })
    saved = true
    setTimeout(() => (saved = false), 2000)
  }

  const store = new BackupsStore()
  let confirmingRestore = $state<string | null>(null)
  let fileInput: HTMLInputElement | undefined = $state()

  function formatSize(bytes: number): string {
    if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(0)} KB`
    return `${(bytes / (1024 * 1024)).toFixed(1)} MB`
  }

  function handleFileChange(e: Event) {
    const file = (e.target as HTMLInputElement).files?.[0]
    if (file) store.restoreUpload(file)
  }
</script>

<section class="space-y-5">
  <h2 class="text-xs font-semibold uppercase tracking-widest text-muted-foreground">
    Backup destination
  </h2>
  <div class="space-y-1.5">
    <p class="text-sm font-medium">Backup directory</p>
    <p class="text-xs text-muted-foreground">
      Point this at durable storage (e.g. a NAS mount). Empty = backups off.
    </p>
    <Input bind:value={backupDir} placeholder="/data/backups" class="max-w-xs font-mono text-xs" />
  </div>
  <SettingRow label="Backup interval (hours)" hint="How often an automatic backup runs">
    <NumberStepper value={intervalHours} min={1} max={168} onChange={(v) => (intervalHours = v)} />
  </SettingRow>
  <Button onclick={handleSave} disabled={saving} class="gap-1.5">
    {#if saving}
      <LoaderCircle class="w-3.5 h-3.5 animate-spin" />
    {:else if saved}
      <Check class="w-3.5 h-3.5" />
    {/if}
    {saved ? 'Saved' : 'Save'}
  </Button>

  <h2 class="text-xs font-semibold uppercase tracking-widest text-muted-foreground pt-4">
    Backups
  </h2>

  {#if store.error}
    <p class="text-sm text-destructive">{store.error}</p>
  {/if}
  {#if store.restoreError}
    <p class="text-sm text-destructive">{store.restoreError}</p>
  {/if}

  <div class="flex gap-2">
    <Button onclick={() => store.backupNow()} disabled={store.backingUp} class="gap-1.5">
      {#if store.backingUp}<LoaderCircle class="w-3.5 h-3.5 animate-spin" />{/if}
      Backup now
    </Button>
    <Button variant="outline" onclick={() => fileInput?.click()} disabled={store.restoring} class="gap-1.5">
      <Upload class="w-3.5 h-3.5" />
      Restore from file
    </Button>
    <input bind:this={fileInput} type="file" accept=".db" class="hidden" onchange={handleFileChange} />
  </div>

  {#if store.loading}
    <LoaderCircle class="w-4 h-4 animate-spin text-muted-foreground" />
  {:else if store.backups.length === 0}
    <p class="text-sm text-muted-foreground">No backups yet.</p>
  {:else}
    <div class="space-y-1.5">
      {#each store.backups as b (b.filename)}
        <div class="flex items-center gap-2 px-3 py-2 rounded-lg bg-muted/50">
          <div class="flex-1 min-w-0">
            <p class="text-sm font-mono truncate">{b.filename}</p>
            <p class="text-xs text-muted-foreground">{b.created_at} · {formatSize(b.size_bytes)}</p>
          </div>

          {#if confirmingRestore === b.filename}
            <p class="text-xs text-destructive mr-2">Restarts the app — sure?</p>
            <Button size="sm" variant="outline" onclick={() => (confirmingRestore = null)}>Cancel</Button>
            <Button size="sm" onclick={() => { store.restore(b.filename); confirmingRestore = null }} disabled={store.restoring}>
              Confirm
            </Button>
          {:else}
            <button
              type="button"
              title="Restore this backup"
              onclick={() => (confirmingRestore = b.filename)}
              class="w-6 h-6 flex items-center justify-center rounded-md text-muted-foreground hover:text-foreground hover:bg-accent transition-colors"
            >
              <RotateCcw class="w-3.5 h-3.5" />
            </button>
            <button
              type="button"
              title="Delete backup"
              onclick={() => store.deleteBackup(b.filename)}
              class="w-6 h-6 flex items-center justify-center rounded-md text-muted-foreground hover:text-destructive hover:bg-destructive/10 transition-colors"
            >
              <Trash2 class="w-3.5 h-3.5" />
            </button>
          {/if}
        </div>
      {/each}
    </div>
  {/if}
</section>
