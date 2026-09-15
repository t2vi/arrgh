<script lang="ts">
  import { api } from '../../lib/api'
  import type { Title } from '../../lib/types'
  import { timeAgo } from './timeAgo'

  let { manga, onClick }: { manga: Title; onClick: () => void } = $props()

  let failed = $state(false)
  const src = $derived(!failed ? (!manga.cover_url?.startsWith('http') ? api.coverUrl(manga.id) : manga.cover_url) : '')

  function handleKeydown(e: KeyboardEvent) {
    if (e.key === 'Enter' || e.key === ' ') onClick()
  }
</script>

<div
  data-nav
  role="button"
  tabindex="0"
  class="flex gap-3 rounded-lg bg-card border border-border p-3 cursor-pointer hover:bg-accent/40 transition-colors"
  onclick={onClick}
  onkeydown={handleKeydown}
>
  <div class="shrink-0 w-11">
    {#if failed || !src}
      <div class="w-full aspect-[2/3] bg-muted rounded text-lg flex items-center justify-center">📖</div>
    {:else}
      <img src={src} alt="" class="w-full aspect-[2/3] object-cover bg-muted rounded" onerror={() => (failed = true)} />
    {/if}
  </div>
  <div class="min-w-0 flex-1">
    <p class="text-sm font-semibold truncate">{manga.title}</p>
    {#if (manga.total_chapters ?? 0) > 0}
      <p class="text-[11px] text-muted-foreground">Chapter {manga.total_chapters} available</p>
    {/if}
    <p class="text-[11px] text-muted-foreground mt-0.5">{timeAgo(manga.updated_at)}</p>
  </div>
</div>
