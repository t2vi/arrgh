<script lang="ts">
  import { api } from '../../lib/api'

  let { mangaId, value }: { mangaId: string; value: string | null } = $props()

  // svelte-ignore state_referenced_locally
  let draft = $state(value ?? '')
  let saving = $state(false)

  async function handleSave(v: string | null) {
    saving = true
    await api.setTitleDownloadDir(mangaId, v).catch(() => {})
    saving = false
  }

  const isDirty = $derived(draft !== (value ?? ''))
  const isDefault = $derived(!value)
</script>

<div class="rounded-lg bg-card border border-border p-4 space-y-3">
  <p class="text-[10px] font-semibold uppercase tracking-wider text-muted-foreground">Download Path</p>
  <div class="flex gap-2">
    <input
      type="text"
      bind:value={draft}
      onkeydown={(e) => {
        if (e.key === 'Enter') handleSave(draft.trim() || null)
      }}
      placeholder="/path/to/manga"
      class="flex-1 bg-muted border border-transparent focus:border-ring rounded-md px-3 py-1.5 text-xs outline-none transition-colors"
    />
    {#if isDirty}
      <button
        onclick={() => handleSave(draft.trim() || null)}
        disabled={saving}
        class="px-3 py-1.5 rounded-md bg-primary text-primary-foreground text-xs font-semibold hover:opacity-90 transition-opacity"
      >
        Save
      </button>
    {/if}
    {#if !isDirty && !isDefault}
      <button
        onclick={() => {
          draft = ''
          handleSave(null)
        }}
        class="px-3 py-1.5 rounded-md bg-muted border border-border text-xs text-muted-foreground hover:text-foreground transition-colors"
      >
        Reset
      </button>
    {/if}
  </div>
  <p class="text-[10px] text-muted-foreground leading-relaxed">
    {isDefault ? 'Uses default: _downloads/{title}/ inside your manga dir.' : `Chapters save to ${value}`}
  </p>
</div>
