<script lang="ts">
  import { api } from '../../lib/api'
  import { cn } from '../../lib/utils'

  const CONTENT_TYPES = ['manga', 'manhwa', 'manhua', 'novel'] as const

  let { mangaId, value }: { mangaId: string; value: string } = $props()

  // svelte-ignore state_referenced_locally
  let current = $state(value)
  let saving = $state(false)

  async function handleChange(next: string) {
    if (next === current) return
    saving = true
    const prev = current
    current = next
    await api.setTitleContentType(mangaId, next).catch(() => (current = prev))
    saving = false
  }
</script>

<div class="rounded-lg bg-card border border-border p-4 space-y-2">
  <p class="text-[10px] font-semibold uppercase tracking-wider text-muted-foreground">Content Type</p>
  <div class="flex flex-wrap gap-1.5">
    {#each CONTENT_TYPES as ct (ct)}
      <button
        type="button"
        disabled={saving}
        onclick={() => handleChange(ct)}
        class={cn(
          'px-2.5 py-1 rounded-md text-xs font-medium capitalize transition-colors',
          current === ct ? 'bg-primary text-primary-foreground' : 'bg-muted text-muted-foreground hover:text-foreground hover:bg-muted/80',
        )}
      >
        {ct}
      </button>
    {/each}
  </div>
  <p class="text-xs text-muted-foreground">Changing type clears incompatible sources and re-matches.</p>
</div>
