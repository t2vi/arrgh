<script lang="ts">
  import type { SearchResult } from '../../lib/api'
  import TrendingCard from './TrendingCard.svelte'

  let {
    heading,
    items,
    loading,
    onSelect,
  }: {
    heading: string
    items: SearchResult[]
    loading: boolean
    onSelect: (r: SearchResult) => void
  } = $props()

  const BADGES = ['HOT', 'TOP', 'NEW', '🔥', '📈', '⭐', '💥', '🎯']
</script>

{#if loading || items.length >= 2}
  <section class="mt-8 px-6 space-y-4">
    <h2 class="text-xl font-bold">{heading}</h2>
    <div class="flex gap-4 overflow-x-auto pb-2 -mx-6 px-6 [&::-webkit-scrollbar]:hidden [-ms-overflow-style:none] [scrollbar-width:none]">
      {#if loading}
        {#each Array.from({ length: 6 }) as _, i (i)}
          <div class="shrink-0 w-36">
            <div class="rounded-xl aspect-[2/3] bg-muted animate-pulse"></div>
            <div class="mt-2 h-3 w-24 bg-muted animate-pulse rounded"></div>
          </div>
        {/each}
      {:else}
        {#each items as r, i (r.mangaupdates_id)}
          <TrendingCard result={r} badge={BADGES[i] ?? '•'} onClick={() => onSelect(r)} />
        {/each}
      {/if}
    </div>
  </section>
{/if}
