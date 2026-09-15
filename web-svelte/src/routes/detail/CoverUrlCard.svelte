<script lang="ts">
  import { api } from '../../lib/api'

  let { mangaId, value, onSaved }: { mangaId: string; value: string | null; onSaved: () => void } = $props()

  // svelte-ignore state_referenced_locally
  let draft = $state(value ?? '')
  let saving = $state(false)

  async function handleSave(v: string) {
    saving = true
    await api.setTitleCoverUrl(mangaId, v).catch(() => {})
    saving = false
    onSaved()
  }

  const isDirty = $derived(draft !== (value ?? ''))
</script>

<div class="rounded-lg bg-card border border-border p-4 space-y-3">
  <p class="text-[10px] font-semibold uppercase tracking-wider text-muted-foreground">Cover URL</p>
  <div class="flex gap-2">
    <input
      type="text"
      bind:value={draft}
      onkeydown={(e) => {
        if (e.key === 'Enter') handleSave(draft.trim())
      }}
      placeholder="https://…"
      class="flex-1 bg-muted border border-transparent focus:border-ring rounded-md px-3 py-1.5 text-xs outline-none transition-colors"
    />
    {#if isDirty}
      <button
        onclick={() => handleSave(draft.trim())}
        disabled={saving}
        class="px-3 py-1.5 rounded-md bg-primary text-primary-foreground text-xs font-semibold hover:opacity-90 transition-opacity"
      >
        Save
      </button>
    {/if}
    {#if !isDirty && value}
      <button
        onclick={() => {
          draft = ''
          handleSave('')
        }}
        class="px-3 py-1.5 rounded-md bg-muted border border-border text-xs text-muted-foreground hover:text-foreground transition-colors"
      >
        Reset
      </button>
    {/if}
  </div>
  <p class="text-[10px] text-muted-foreground leading-relaxed">Override cover with any direct image URL.</p>
</div>
