<script lang="ts">
  import { LoaderCircle, Download, RotateCcw, TriangleAlert } from '@lucide/svelte'
  import { api, type PluginStatusList } from '../../lib/api'

  let data = $state<PluginStatusList | null>(null)
  let loadError = $state('')
  let busy = $state<string | null>(null)
  let actionError = $state<{ id: string; message: string } | null>(null)

  // Catalog-only entries are installs (Browse plugins), not rows here.
  const loaded = $derived(data?.plugins.filter((p) => p.origin) ?? [])

  function reload() {
    api.listPlugins()
      .then((d) => { data = d; loadError = '' })
      .catch(() => (loadError = 'Failed to load plugins.'))
  }

  reload()

  async function run(id: string, action: (id: string) => Promise<void>) {
    busy = id
    actionError = null
    try {
      await action(id)
      reload()
    } catch (err: unknown) {
      // "422 checksum mismatch: …" → the reason after the status code
      const msg = err instanceof Error ? err.message.replace(/^\d{3}\s*/, '') : ''
      actionError = { id, message: msg || 'Failed.' }
    } finally {
      busy = null
    }
  }
</script>

<section class="space-y-3">
  <span class="text-xs font-medium text-muted-foreground">Plugins</span>

  {#if data?.catalog === 'fallback'}
    <p class="flex items-center gap-1.5 text-xs text-muted-foreground">
      <TriangleAlert class="w-3.5 h-3.5 shrink-0" />
      Plugin catalog unreachable — showing the copy shipped with this version of *ARRgh.
    </p>
  {:else if data && data.catalog === null}
    <p class="flex items-center gap-1.5 text-xs text-muted-foreground">
      <TriangleAlert class="w-3.5 h-3.5 shrink-0" />
      The plugin catalog is unavailable, so updates can't be checked. Installed plugins keep working.
    </p>
  {/if}

  {#if loadError}
    <p class="text-sm text-destructive">{loadError}</p>
  {:else if !data}
    <LoaderCircle class="w-4 h-4 animate-spin text-muted-foreground" />
  {:else}
    <div class="space-y-1.5">
      {#each loaded as p (p.id)}
        {@const isBusy = busy === p.id}
        <div class="px-3 py-2 rounded-lg bg-muted/50">
          <div class="flex items-center gap-2">
            <div class="flex-1 min-w-0 flex items-center gap-2 flex-wrap">
              <p class="text-sm font-medium truncate">{p.name}</p>
              {#if p.update_available}
                <span class="text-[10px] text-primary">{p.loaded_version ?? 'unknown'} → {p.catalog_version}</span>
              {:else}
                <span class="text-[10px] text-muted-foreground">{p.loaded_version ? `v${p.loaded_version}` : 'version unknown'}</span>
              {/if}
              <span class="px-1.5 rounded-full bg-card border border-border text-[10px] text-muted-foreground">
                {p.origin === 'downloaded' ? 'updated' : 'bundled'}
              </span>
            </div>

            {#if p.update_available}
              <button
                type="button"
                aria-label="Update {p.name}"
                title={p.blocked_reason ?? `Update to ${p.catalog_version}`}
                disabled={isBusy || !!p.blocked_reason}
                onclick={() => run(p.id, api.updatePlugin)}
                class="flex items-center gap-1 px-2.5 py-1 rounded-md bg-primary text-primary-foreground text-xs font-medium hover:bg-primary/90 disabled:opacity-50 transition-colors shrink-0"
              >
                {#if isBusy}<LoaderCircle class="w-3 h-3 animate-spin" />{:else}<Download class="w-3 h-3" />{/if}
                Update
              </button>
            {/if}
            {#if p.origin === 'downloaded' && p.has_bundled}
              <button
                type="button"
                aria-label="Revert {p.name}"
                title="Go back to the version bundled with *ARRgh"
                disabled={isBusy}
                onclick={() => run(p.id, api.revertPlugin)}
                class="flex items-center gap-1 px-2.5 py-1 rounded-md bg-muted border border-border text-xs font-medium text-foreground hover:bg-muted/70 disabled:opacity-50 transition-colors shrink-0"
              >
                <RotateCcw class="w-3 h-3" />
                Revert
              </button>
            {/if}
          </div>
          {#if p.update_available && p.blocked_reason}
            <p class="text-xs text-muted-foreground mt-1">Can't update: {p.blocked_reason}</p>
          {/if}
          {#if actionError?.id === p.id}
            <p class="text-xs text-destructive mt-1">{actionError.message}</p>
          {/if}
        </div>
      {/each}
    </div>
  {/if}
</section>
