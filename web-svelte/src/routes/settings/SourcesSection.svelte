<script lang="ts">
  import { LoaderCircle, Globe, KeyRound, Plus, Trash2, PackageSearch, Download, CircleCheck, X } from '@lucide/svelte'
  import { api, type SourceRow, type PluginIndexEntry } from '../../lib/api'
  import { cn } from '../../lib/utils'

  let sources = $state<SourceRow[]>([])
  let loading = $state(true)
  let url = $state('')
  let apiKey = $state('')
  let adding = $state(false)
  let addError = $state('')
  let browseOpen = $state(false)

  let browseEntries = $state<PluginIndexEntry[]>([])
  let browseLoading = $state(true)
  let installing = $state<string | null>(null)
  let browseError = $state('')

  function reload() {
    loading = true
    api.listSources()
      .then((d) => (sources = d))
      .catch(() => {})
      .finally(() => (loading = false))
  }

  reload()

  async function handleAdd(e: SubmitEvent) {
    e.preventDefault()
    addError = ''
    adding = true
    try {
      await api.addSource(url.trim(), apiKey.trim() || undefined)
      url = ''
      apiKey = ''
      reload()
    } catch (err: unknown) {
      const msg = err instanceof Error ? err.message : ''
      addError = msg.includes('502') ? 'Could not reach plugin — check the URL.' : 'Failed to add source.'
    } finally {
      adding = false
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

  async function handleInstall(pluginId: string) {
    installing = pluginId
    try {
      await api.installPlugin(pluginId)
      reload()
    } finally {
      installing = null
    }
  }

  function openBrowse() {
    browseOpen = true
    browseLoading = true
    browseError = ''
    api.listPluginIndex()
      .then((d) => (browseEntries = d))
      .catch(() => (browseError = 'Failed to load plugin index.'))
      .finally(() => (browseLoading = false))
  }

  const installedPluginIds = $derived(new Set(
    sources.map((s) => {
      const parts = s.base_url.split('/')
      return parts[parts.length - 1]
    })
  ))
</script>

<section class="space-y-4">
  <p class="text-xs text-muted-foreground">
    External source plugins extend *ARRgh with additional manga and novel sources.
  </p>

  <div class="flex items-center justify-between">
    <span class="text-xs font-medium text-muted-foreground">Installed sources</span>
    <button
      type="button"
      onclick={openBrowse}
      class="flex items-center gap-1.5 px-2.5 py-1 rounded-md bg-muted border border-border text-xs font-medium text-foreground hover:bg-muted/70 transition-colors"
    >
      <PackageSearch class="w-3.5 h-3.5 text-primary" />
      Browse plugins
    </button>
  </div>

  {#if loading}
    <LoaderCircle class="w-4 h-4 animate-spin text-muted-foreground" />
  {:else if sources.length === 0}
    <p class="text-sm text-muted-foreground">No external sources yet.</p>
  {:else}
    <div class="space-y-1.5">
      {#each sources as source (source.id)}
        <div class="flex items-center gap-2 px-3 py-2 rounded-lg bg-muted/50">
          <div class="flex-1 min-w-0">
            <div class="flex items-center gap-2">
              <p class="text-sm font-medium truncate">{source.name}</p>
              {#if source.has_api_key}
                <span title="Uses API key"><KeyRound class="w-3 h-3 text-muted-foreground shrink-0" /></span>
              {/if}
              {#if source.is_community}
                <span class="px-1.5 py-0 rounded-full bg-primary/15 text-primary text-[10px] font-medium shrink-0">community</span>
              {/if}
            </div>
            <div class="flex items-center gap-1.5 mt-0.5 flex-wrap">
              <p class="text-xs text-muted-foreground truncate">{source.base_url}</p>
              {#each source.content_types as ct (ct)}
                <span class="px-1.5 py-0 rounded-full bg-card border border-border text-[10px] text-muted-foreground">{ct}</span>
              {/each}
            </div>
          </div>

          <button
            type="button"
            title={source.enabled ? 'Enabled — click to disable' : 'Disabled — click to enable'}
            onclick={() => handleToggle(source.id, source.enabled)}
            class={cn(
              'relative w-8 h-[18px] rounded-full transition-colors shrink-0 overflow-hidden',
              source.enabled ? 'bg-primary' : 'bg-muted border border-border',
            )}
          >
            <span class={cn(
              'absolute top-[2px] left-0 w-3.5 h-3.5 rounded-full bg-white shadow transition-transform',
              source.enabled ? 'translate-x-[16px]' : 'translate-x-[2px]',
            )}></span>
          </button>

          {#if source.is_community}
            <button
              type="button"
              title="Remove plugin"
              onclick={() => handleDelete(source.id)}
              class="w-6 h-6 flex items-center justify-center rounded-md text-muted-foreground hover:text-destructive hover:bg-destructive/10 transition-colors"
            >
              <Trash2 class="w-3.5 h-3.5" />
            </button>
          {/if}
        </div>
      {/each}
    </div>
  {/if}

  <form onsubmit={handleAdd} class="space-y-2 pt-2 border-t border-border">
    <p class="text-xs font-medium text-muted-foreground">Add custom source</p>
    <div class="flex gap-2">
      <div class="relative flex-1">
        <Globe class="absolute left-2.5 top-1/2 -translate-y-1/2 w-3.5 h-3.5 text-muted-foreground pointer-events-none" />
        <input
          class="w-full pl-8 pr-3 py-1.5 rounded-md bg-muted border border-border text-sm placeholder:text-muted-foreground focus:outline-none focus:ring-1 focus:ring-ring"
          placeholder="http://localhost:4000"
          bind:value={url}
          required
        />
      </div>
      <div class="relative w-40">
        <KeyRound class="absolute left-2.5 top-1/2 -translate-y-1/2 w-3.5 h-3.5 text-muted-foreground pointer-events-none" />
        <input
          class="w-full pl-8 pr-3 py-1.5 rounded-md bg-muted border border-border text-sm placeholder:text-muted-foreground focus:outline-none focus:ring-1 focus:ring-ring"
          placeholder="API key (opt.)"
          bind:value={apiKey}
        />
      </div>
      <button
        type="submit"
        disabled={adding}
        class="flex items-center gap-1 px-3 py-1.5 rounded-md bg-primary text-primary-foreground text-sm font-medium hover:bg-primary/90 disabled:opacity-60 transition-colors"
      >
        {#if adding}
          <LoaderCircle class="w-3.5 h-3.5 animate-spin" />
        {:else}
          <Plus class="w-3.5 h-3.5" />
        {/if}
        Add
      </button>
    </div>
    {#if addError}
      <p class="text-xs text-destructive">{addError}</p>
    {/if}
  </form>

  {#if browseOpen}
    <div
      class="fixed inset-0 z-50 flex items-center justify-center bg-black/60 backdrop-blur-sm"
      role="presentation"
      onclick={(e) => { if (e.target === e.currentTarget) browseOpen = false }}
    >
      <div class="relative w-full max-w-lg mx-4 bg-card border border-border rounded-2xl shadow-2xl overflow-hidden">
        <div class="flex items-center justify-between px-5 py-4 border-b border-border">
          <div class="flex items-center gap-2">
            <PackageSearch class="w-4 h-4 text-primary" />
            <h2 class="text-sm font-semibold">Browse Plugins</h2>
          </div>
          <button
            type="button"
            onclick={() => (browseOpen = false)}
            class="w-6 h-6 flex items-center justify-center rounded-md text-muted-foreground hover:text-foreground hover:bg-muted transition-colors"
          >
            <X class="w-4 h-4" />
          </button>
        </div>

        <div class="max-h-[60vh] overflow-y-auto p-4 space-y-2">
          {#if browseLoading}
            <div class="flex justify-center py-8">
              <LoaderCircle class="w-5 h-5 animate-spin text-muted-foreground" />
            </div>
          {:else if browseError}
            <p class="text-sm text-destructive text-center py-4">{browseError}</p>
          {:else if browseEntries.length === 0}
            <p class="text-sm text-muted-foreground text-center py-4">No plugins found.</p>
          {:else}
            {#each browseEntries as entry (entry.id)}
              {@const isInstalled = installedPluginIds.has(entry.id)}
              {@const isBusy = installing === entry.id}
              {@const canInstall = !entry.bundled && !!entry.download_url && !isInstalled}
              <div class="flex items-start gap-3 px-3 py-2.5 rounded-lg bg-muted/40 hover:bg-muted/70 transition-colors">
                <div class="flex-1 min-w-0">
                  <div class="flex items-center gap-2 flex-wrap">
                    <p class="text-sm font-medium">{entry.name}</p>
                    <span class="text-[10px] text-muted-foreground">v{entry.version}</span>
                    {#if entry.bundled}
                      <span class="px-1.5 rounded-full bg-muted border border-border text-[10px] text-muted-foreground">bundled</span>
                    {/if}
                    {#if entry.default_explicit}
                      <span class="px-1.5 rounded-full bg-destructive/15 text-destructive text-[10px]">18+</span>
                    {/if}
                    {#each entry.content_types as ct (ct)}
                      <span class="px-1.5 rounded-full bg-card border border-border text-[10px] text-muted-foreground">{ct}</span>
                    {/each}
                  </div>
                  {#if entry.description}
                    <p class="text-xs text-muted-foreground mt-0.5">{entry.description}</p>
                  {/if}
                </div>

                {#if isInstalled}
                  <div class="flex items-center gap-1 text-xs text-primary shrink-0 mt-0.5">
                    <CircleCheck class="w-3.5 h-3.5" />
                    <span>Installed</span>
                  </div>
                {:else if canInstall}
                  <button
                    type="button"
                    disabled={isBusy}
                    onclick={() => handleInstall(entry.id)}
                    class="flex items-center gap-1 px-2.5 py-1 rounded-md bg-primary text-primary-foreground text-xs font-medium hover:bg-primary/90 disabled:opacity-60 transition-colors shrink-0"
                  >
                    {#if isBusy}
                      <LoaderCircle class="w-3 h-3 animate-spin" />
                    {:else}
                      <Download class="w-3 h-3" />
                    {/if}
                    Install
                  </button>
                {/if}
              </div>
            {/each}
          {/if}
        </div>
      </div>
    </div>
  {/if}
</section>
