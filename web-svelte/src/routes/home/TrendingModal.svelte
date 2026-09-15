<script lang="ts">
  import { X, CheckCircle2, Plus, LoaderCircle } from '@lucide/svelte'
  import { api, type SearchResult } from '../../lib/api'
  import Button from '../../lib/components/ui/Button.svelte'

  let {
    result,
    onClose,
    onViewDetails,
    onAdded,
  }: {
    result: SearchResult
    onClose: () => void
    onViewDetails: (libraryId: string) => void
    onAdded: () => void
  } = $props()

  let coverFailed = $state(false)
  const coverUrl = $derived(result.cover_url ? api.proxyImageUrl(result.cover_url) : '')

  let addPending = $state(false)

  async function handleAdd() {
    addPending = true
    try {
      await api.addTitle(result)
      onAdded()
    } catch {
      // ignore
    } finally {
      addPending = false
    }
  }

  const tags = $derived(result.tags?.split(',').filter(Boolean) ?? [])
</script>

<div
  role="dialog"
  aria-modal="true"
  tabindex="-1"
  class="fixed inset-0 z-50 flex items-end sm:items-center justify-center p-4 bg-black/60 backdrop-blur-sm"
  onclick={(e) => e.target === e.currentTarget && onClose()}
  onkeydown={(e) => e.key === 'Escape' && onClose()}
>
  <div class="relative w-full max-w-lg bg-card rounded-2xl overflow-hidden shadow-2xl border border-border animate-in fade-in slide-in-from-bottom-4 duration-200">
    <button
      onclick={onClose}
      class="absolute top-3 right-3 z-10 p-1.5 rounded-full bg-black/40 text-white/70 hover:text-white hover:bg-black/60 transition-colors"
    >
      <X class="w-4 h-4" />
    </button>

    <div class="relative h-48 overflow-hidden">
      {#if coverFailed || !coverUrl}
        <div class="w-full h-full bg-muted flex items-center justify-center text-6xl">📖</div>
      {:else}
        <img src={coverUrl} alt="" class="w-full h-full object-cover object-top" onerror={() => (coverFailed = true)} />
      {/if}
      <div class="absolute inset-0 bg-gradient-to-t from-card via-card/30 to-transparent"></div>
      <div class="absolute bottom-0 left-0 right-0 px-5 pb-4">
        <h2 class="text-xl font-extrabold leading-tight text-white drop-shadow">{result.title}</h2>
      </div>
    </div>

    <div class="px-5 pt-3 pb-5 space-y-4">
      <div class="flex items-center gap-3 flex-wrap">
        {#if result.status && result.status !== 'unknown'}
          <span class="text-[11px] font-semibold px-2 py-0.5 rounded-full bg-primary/15 text-primary border border-primary/20">
            {result.status}
          </span>
        {/if}
        {#if result.author}
          <span class="text-[11px] text-muted-foreground">{result.author}</span>
        {/if}
        {#if result.in_library}
          <span class="flex items-center gap-1 text-[11px] font-semibold text-emerald-400">
            <CheckCircle2 class="w-3.5 h-3.5" /> In library
          </span>
        {/if}
      </div>

      {#if result.description}
        <p class="text-sm text-muted-foreground leading-relaxed line-clamp-4">{result.description}</p>
      {:else}
        <p class="text-sm text-muted-foreground italic">No description available.</p>
      {/if}

      {#if tags.length > 0}
        <div class="flex flex-wrap gap-1.5">
          {#each tags.slice(0, 6) as tag (tag)}
            <span class="text-[10px] px-2 py-0.5 rounded-full bg-muted text-muted-foreground border border-border">
              {tag}
            </span>
          {/each}
        </div>
      {/if}

      <div class="flex gap-2 pt-1">
        {#if result.in_library && result.library_id}
          <Button class="flex-1" onclick={() => onViewDetails(result.library_id as string)}>View Details</Button>
        {:else}
          <Button class="flex-1" onclick={handleAdd} disabled={addPending}>
            {#if addPending}
              <LoaderCircle class="w-4 h-4 mr-2 animate-spin" /> Adding…
            {:else}
              <Plus class="w-4 h-4 mr-2" /> Add to Library
            {/if}
          </Button>
        {/if}
        <Button variant="outline" onclick={onClose} class="px-4">Close</Button>
      </div>
    </div>
  </div>
</div>
