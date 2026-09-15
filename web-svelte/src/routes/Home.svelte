<script lang="ts">
  import { Plus } from '@lucide/svelte'
  import Button from '../lib/components/ui/Button.svelte'
  import { router } from '../lib/router.svelte'
  import { ROUTES } from '../lib/routes'
  import { HomeStore } from '../lib/home.svelte'
  import HomeSkeleton from './home/HomeSkeleton.svelte'
  import GreetingJumbotron from './home/GreetingJumbotron.svelte'
  import ContinueCard from './home/ContinueCard.svelte'
  import LibraryCoverCard from './home/LibraryCoverCard.svelte'
  import NewReleaseCard from './home/NewReleaseCard.svelte'
  import RecentCard from './home/RecentCard.svelte'
  import TrendingLane from './home/TrendingLane.svelte'
  import TrendingModal from './home/TrendingModal.svelte'

  let _params: Record<string, string> = $props()

  const store = new HomeStore()

  const typeCounts = $derived(
    store.items.reduce<Record<string, number>>((acc, m) => {
      acc[m.content_type] = (acc[m.content_type] ?? 0) + 1
      return acc
    }, {}),
  )
</script>

<div class="flex-1 overflow-auto">
  {#if store.isLoading}
    <HomeSkeleton />
  {:else}
    <div class="pb-12">
      {#if store.items.length === 0}
        <div class="flex flex-col items-center justify-center py-24 gap-4">
          <p class="text-muted-foreground text-sm">Library empty — discover titles to add some.</p>
          <Button onclick={() => router.navigate(ROUTES.discover)}>Discover</Button>
        </div>
      {:else}
        <GreetingJumbotron {typeCounts} totalRead={store.totalRead} coverManga={store.coverManga} />

        {#if store.continueItems.length > 0}
          <section class="mt-8 px-6 space-y-4">
            <h2 class="text-xl font-bold">Continue Reading</h2>
            <div class="flex gap-4 overflow-x-auto pb-2 -mx-6 px-6 [&::-webkit-scrollbar]:hidden [-ms-overflow-style:none] [scrollbar-width:none]">
              {#each store.continueItems as item (item.chapter_id)}
                <ContinueCard
                  {item}
                  onPlay={() => router.navigate(ROUTES.reader(item.chapter_id))}
                  onDetail={() => router.navigate(ROUTES.title(item.title_id))}
                />
              {/each}
            </div>
          </section>
        {/if}

        <section class="mt-8 px-6 space-y-4">
          <div class="flex items-center justify-between">
            <h2 class="text-xl font-bold">My Library</h2>
            <button
              onclick={() => router.navigate(ROUTES.library)}
              class="text-sm text-primary hover:opacity-80 transition-opacity font-medium"
            >
              View All
            </button>
          </div>
          <div class="flex gap-4 overflow-x-auto pb-2 -mx-6 px-6 [&::-webkit-scrollbar]:hidden [-ms-overflow-style:none] [scrollbar-width:none]">
            {#each store.items.slice(0, 10) as m (m.id)}
              <LibraryCoverCard manga={m} onClick={() => router.navigate(ROUTES.title(m.id))} />
            {/each}
          </div>
        </section>

        {#if store.newReleases.length > 0}
          <section class="mt-8 px-6 space-y-4">
            <div class="flex items-center justify-between">
              <h2 class="text-xl font-bold">New Releases</h2>
              <span class="text-xs text-muted-foreground">{store.newReleases.length} new</span>
            </div>
            <div class="flex gap-3 overflow-x-auto pb-2 -mx-6 px-6 [&::-webkit-scrollbar]:hidden [-ms-overflow-style:none] [scrollbar-width:none]">
              {#each store.newReleases as r (r.chapter_id)}
                <NewReleaseCard
                  item={r}
                  onClick={() => router.navigate(ROUTES.reader(r.chapter_id))}
                  onMangaClick={() => router.navigate(ROUTES.title(r.manga_id))}
                />
              {/each}
            </div>
          </section>
        {/if}

        {#if store.recentUp.length > 0}
          <section class="mt-8 px-6 space-y-4">
            <h2 class="text-xl font-bold">Recently Updated</h2>
            <div class="grid grid-cols-3 gap-3">
              {#each store.recentUp as m (m.id)}
                <RecentCard manga={m} onClick={() => router.navigate(ROUTES.title(m.id))} />
              {/each}
            </div>
          </section>
        {/if}
      {/if}

      <TrendingLane
        heading="Trending Manga"
        items={store.trendingManga}
        loading={store.trendingMangaLoading}
        onSelect={(r) => (store.selectedTrending = r)}
      />
      <TrendingLane
        heading="Trending Manhwa"
        items={store.trendingManhwa}
        loading={store.trendingManhwaLoading}
        onSelect={(r) => (store.selectedTrending = r)}
      />
      <TrendingLane
        heading="Trending Manhua"
        items={store.trendingManhua}
        loading={store.trendingManhuaLoading}
        onSelect={(r) => (store.selectedTrending = r)}
      />
      <TrendingLane
        heading="Trending Adult Manhwa"
        items={store.trendingAdultManhwa}
        loading={store.trendingAdultManhwaLoading}
        onSelect={(r) => (store.selectedTrending = r)}
      />
    </div>
  {/if}
</div>

<button
  onclick={() => router.navigate(ROUTES.discover)}
  title="Discover manga"
  class="fixed bottom-6 right-6 w-12 h-12 rounded-full bg-primary text-primary-foreground flex items-center justify-center shadow-xl hover:opacity-90 transition-opacity z-10"
>
  <Plus class="w-5 h-5" />
</button>

{#if store.selectedTrending}
  <TrendingModal
    result={store.selectedTrending}
    onClose={() => (store.selectedTrending = null)}
    onViewDetails={(id) => {
      store.selectedTrending = null
      router.navigate(ROUTES.title(id))
    }}
    onAdded={() => {
      store.selectedTrending = null
      router.navigate(ROUTES.library)
    }}
  />
{/if}
