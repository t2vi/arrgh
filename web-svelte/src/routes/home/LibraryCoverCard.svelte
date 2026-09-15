<script lang="ts">
  import { LoaderCircle } from '@lucide/svelte'
  import { api } from '../../lib/api'
  import type { Title } from '../../lib/types'

  let { manga, onClick }: { manga: Title; onClick: () => void } = $props()

  let failed = $state(false)
  const src = $derived(!failed ? (!manga.cover_url?.startsWith('http') ? api.coverUrl(manga.id) : manga.cover_url) : '')
  const tags = $derived(manga.tags?.split(', ').slice(0, 2).join(', ') ?? '')

  function handleKeydown(e: KeyboardEvent) {
    if (e.key === 'Enter' || e.key === ' ') onClick()
  }
</script>

<div data-nav role="button" tabindex="0" class="shrink-0 w-36 cursor-pointer group" onclick={onClick} onkeydown={handleKeydown}>
  <div class="relative rounded-xl overflow-hidden transition-all duration-300 group-hover:scale-[1.04] group-hover:shadow-[0_0_20px_rgba(139,92,246,0.35)]">
    {#if failed || !src}
      <div class="w-full aspect-[2/3] bg-muted flex items-center justify-center text-3xl">📖</div>
    {:else}
      <img src={src} alt={manga.title} class="w-full aspect-[2/3] object-cover bg-muted block" onerror={() => (failed = true)} />
    {/if}
    <div class="absolute inset-x-0 bottom-0 h-1/3 bg-gradient-to-t from-black/70 to-transparent opacity-0 group-hover:opacity-100 transition-opacity pointer-events-none"></div>
    {#if manga.sync_status === 'syncing'}
      <div class="absolute inset-0 flex items-center justify-center bg-black/40">
        <LoaderCircle class="w-5 h-5 animate-spin text-primary" />
      </div>
    {/if}
  </div>
  <div class="mt-2 px-0.5">
    <p class="text-sm font-semibold line-clamp-1">{manga.title}</p>
    {#if manga.is_explicit}
      <span class="inline-flex items-center px-1.5 py-px rounded text-[10px] font-bold bg-red-500/15 text-red-400 mt-0.5">18+</span>
    {/if}
    {#if tags}
      <p class="text-[11px] text-muted-foreground truncate mt-0.5">{tags}</p>
    {/if}
  </div>
</div>
