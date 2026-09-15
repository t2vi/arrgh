<script lang="ts">
  import { Play } from '@lucide/svelte'
  import { api, type ContinueItem } from '../../lib/api'

  let { item, onPlay, onDetail }: { item: ContinueItem; onPlay: () => void; onDetail: () => void } = $props()

  let failed = $state(false)
  const src = $derived(
    !failed
      ? item.cover_url?.startsWith('http')
        ? item.cover_url
        : item.cover_url
          ? api.coverUrl(item.title_id)
          : ''
      : '',
  )
  const pct = $derived(item.total_chapters > 0 ? Math.round((item.chapters_read / item.total_chapters) * 100) : 0)

  function handleKeydown(e: KeyboardEvent) {
    if (e.key === 'Enter' || e.key === ' ') onDetail()
  }
</script>

<div
  data-nav
  role="button"
  tabindex="0"
  class="shrink-0 w-36 flex flex-col gap-2 cursor-pointer group"
  onclick={onDetail}
  onkeydown={handleKeydown}
>
  <div class="relative rounded-xl overflow-hidden aspect-[2/3] transition-all duration-300 group-hover:scale-[1.04] group-hover:shadow-[0_0_20px_rgba(139,92,246,0.35)]">
    {#if !src}
      <div class="w-full h-full bg-muted flex items-center justify-center text-3xl">📖</div>
    {:else}
      <img src={src} alt="" class="w-full h-full object-cover bg-muted" onerror={() => (failed = true)} />
    {/if}
    <div class="absolute bottom-0 left-0 right-0 h-1 bg-black/40">
      <div class="h-full bg-primary transition-all" style={`width: ${pct}%`}></div>
    </div>
    <button
      onclick={(e) => {
        e.stopPropagation()
        onPlay()
      }}
      class="absolute inset-0 flex items-center justify-center bg-black/0 group-hover:bg-black/30 transition-colors"
    >
      <div class="w-10 h-10 rounded-full bg-primary/80 flex items-center justify-center opacity-0 group-hover:opacity-100 transition-opacity shadow-lg">
        <Play class="w-4 h-4 fill-white text-white ml-0.5" />
      </div>
    </button>
  </div>
  <div class="px-0.5">
    <p class="text-xs font-semibold line-clamp-1">{item.manga_title}</p>
    <p class="text-[10px] text-primary font-medium mt-0.5">
      Ch. {Number.isInteger(item.chapter_number) ? item.chapter_number : item.chapter_number.toFixed(1)}
    </p>
    <p class="text-[10px] text-muted-foreground">{item.chapters_read}/{item.total_chapters} read</p>
  </div>
</div>
