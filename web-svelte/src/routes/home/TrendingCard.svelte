<script lang="ts">
  import { api, type SearchResult } from '../../lib/api'

  let { result, badge, onClick }: { result: SearchResult; badge: string; onClick: () => void } = $props()

  let failed = $state(false)
  const src = $derived(failed ? '' : result.cover_url ? api.proxyImageUrl(result.cover_url) : '')
  const coverLoading = $derived(!result.cover_url && !failed)

  function handleKeydown(e: KeyboardEvent) {
    if (e.key === 'Enter' || e.key === ' ') onClick()
  }
</script>

<div data-nav role="button" tabindex="0" class="shrink-0 w-36 cursor-pointer group" onclick={onClick} onkeydown={handleKeydown}>
  <div class="relative rounded-xl overflow-hidden aspect-[2/3] transition-all duration-300 group-hover:scale-[1.04] group-hover:shadow-[0_0_20px_rgba(139,92,246,0.35)]">
    {#if coverLoading}
      <div class="w-full h-full bg-muted animate-pulse"></div>
    {:else if !src}
      <div class="w-full h-full bg-muted flex items-center justify-center text-3xl">📖</div>
    {:else}
      <img src={src} alt="" class="w-full h-full object-cover object-top" onerror={() => (failed = true)} />
    {/if}
    <div class="absolute top-2 left-2 px-1.5 py-0.5 rounded bg-primary text-primary-foreground text-[9px] font-black leading-none">
      {badge}
    </div>
    {#if result.in_library}
      <div class="absolute top-2 right-2 w-2 h-2 rounded-full bg-emerald-400 shadow"></div>
    {/if}
  </div>
  <div class="mt-2 px-0.5">
    <p class="text-sm font-semibold line-clamp-1">{result.title}</p>
    <p class="text-[10px] text-muted-foreground mt-0.5 line-clamp-1">{result.author ?? result.content_type}</p>
    {#if result.is_explicit}
      <span class="inline-flex items-center px-1.5 py-px rounded text-[10px] font-bold bg-red-500/15 text-red-400 mt-0.5">18+</span>
    {/if}
    {#if result.in_library}
      <p class="text-[11px] text-primary mt-0.5 font-medium">In library</p>
    {/if}
  </div>
</div>
