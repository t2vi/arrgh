<script lang="ts">
  import { Download, Play } from '@lucide/svelte'
  import { api, type NewReleaseItem } from '../../lib/api'
  import { timeAgo } from './timeAgo'

  let {
    item,
    onClick,
    onMangaClick,
  }: { item: NewReleaseItem; onClick: () => void; onMangaClick: () => void } = $props()

  let failed = $state(false)
  const src = $derived(
    !failed
      ? item.cover_url?.startsWith('http')
        ? item.cover_url
        : item.cover_url
          ? api.coverUrl(item.manga_id)
          : ''
      : '',
  )
  const chNum = $derived(Number.isInteger(item.chapter_number) ? item.chapter_number : item.chapter_number.toFixed(1))

  function handleKeydown(e: KeyboardEvent) {
    if (e.key === 'Enter' || e.key === ' ') onClick()
  }
</script>

<div class="shrink-0 w-32 flex flex-col gap-2">
  <div
    data-nav
    role="button"
    tabindex="0"
    class="relative rounded-xl overflow-hidden cursor-pointer group transition-all duration-300 hover:scale-[1.04] hover:shadow-[0_0_18px_rgba(139,92,246,0.35)]"
    onclick={onClick}
    onkeydown={handleKeydown}
  >
    {#if !src}
      <div class="w-full aspect-[2/3] bg-muted flex items-center justify-center text-2xl">📖</div>
    {:else}
      <img src={src} alt="" class="w-full aspect-[2/3] object-cover bg-muted block" onerror={() => (failed = true)} />
    {/if}
    <div class="absolute top-2 left-2 px-1.5 py-0.5 rounded-md bg-primary text-primary-foreground text-[10px] font-black leading-none">
      Ch.{chNum}
    </div>
    {#if item.downloaded}
      <div class="absolute top-2 right-2 p-0.5 rounded-full bg-black/50">
        <Download class="w-2.5 h-2.5 text-emerald-400" />
      </div>
    {/if}
    <div class="absolute inset-0 bg-black/0 group-hover:bg-black/20 flex items-center justify-center transition-colors">
      <Play class="w-6 h-6 text-white opacity-0 group-hover:opacity-100 transition-opacity fill-current drop-shadow" />
    </div>
  </div>
  <button onclick={onMangaClick} class="text-left px-0.5">
    <p class="text-xs font-semibold line-clamp-1 hover:text-primary transition-colors">{item.manga_title}</p>
    <p class="text-[10px] text-muted-foreground mt-0.5">{timeAgo(item.chapter_created_at)}</p>
  </button>
</div>
