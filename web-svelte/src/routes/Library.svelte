<script lang="ts">
  import { Search, SlidersHorizontal, Plus } from '@lucide/svelte'
  import Input from '../lib/components/ui/Input.svelte'
  import Button from '../lib/components/ui/Button.svelte'
  import { cn } from '../lib/utils'
  import { router } from '../lib/router.svelte'
  import { ROUTES } from '../lib/routes'
  import { LibraryStore } from '../lib/library.svelte'
  import MangaCard from './library/MangaCard.svelte'
  import MangaGridSkeleton from './library/MangaGridSkeleton.svelte'
  import SortDropdown from './library/SortDropdown.svelte'

  let _params: Record<string, string> = $props()

  const CONTENT_TYPE_OPTIONS = [
    { value: 'manga', label: 'Manga' },
    { value: 'manhwa', label: 'Manhwa' },
    { value: 'manhua', label: 'Manhua' },
    { value: 'novel', label: 'Novel' },
  ]

  const STATUS_OPTIONS = [
    { value: 'ongoing', label: 'Ongoing' },
    { value: 'completed', label: 'Completed' },
    { value: 'hiatus', label: 'Hiatus' },
    { value: 'cancelled', label: 'Cancelled' },
  ]

  const store = new LibraryStore()
</script>

<header class="flex items-center gap-4 px-6 py-3 border-b border-border shrink-0 bg-background/80 backdrop-blur-xl sticky top-0 z-10">
  <div class="flex-1 max-w-sm">
    <div class="relative">
      <Search class="absolute left-3 top-1/2 -translate-y-1/2 w-4 h-4 text-muted-foreground pointer-events-none" />
      <Input
        class="pl-9 rounded-full bg-muted border-transparent focus-visible:border-ring"
        placeholder="Search your library…"
        value={store.search}
        oninput={(e: Event) => {
          store.search = (e.currentTarget as HTMLInputElement).value
          store.setPage(1)
        }}
      />
    </div>
  </div>

  <div class="flex-1"></div>

  <div class="flex items-center gap-2 shrink-0">
    <SortDropdown value={store.sort} onChange={(v) => store.setSort(v)} />
    <Button
      variant="outline"
      size="sm"
      class={cn('gap-1.5 text-xs', store.hasFilters && 'border-primary text-primary')}
      onclick={() => (store.showFilters = !store.showFilters)}
    >
      <SlidersHorizontal class="w-3 h-3" />
      Filters
      {#if store.hasFilters}
        <span class="bg-primary text-primary-foreground rounded-full w-4 h-4 text-[10px] flex items-center justify-center leading-none">
          {store.contentTypes.length + store.statuses.length}
        </span>
      {/if}
    </Button>
  </div>
</header>

{#if store.showFilters}
  <div class="px-6 py-3 border-b border-border bg-muted/30 flex items-start gap-6 flex-wrap shrink-0">
    <div class="flex items-center gap-2 flex-wrap">
      <span class="text-xs text-muted-foreground shrink-0">Type</span>
      {#each CONTENT_TYPE_OPTIONS as { value, label } (value)}
        <button
          onclick={() => store.toggleContentType(value)}
          class={cn(
            'px-2.5 py-0.5 rounded-full text-xs border transition-colors',
            store.contentTypes.includes(value)
              ? 'bg-primary text-primary-foreground border-primary'
              : 'border-border text-muted-foreground hover:border-foreground hover:text-foreground',
          )}
        >
          {label}
        </button>
      {/each}
    </div>
    <div class="flex items-center gap-2 flex-wrap">
      <span class="text-xs text-muted-foreground shrink-0">Status</span>
      {#each STATUS_OPTIONS as { value, label } (value)}
        <button
          onclick={() => store.toggleStatus(value)}
          class={cn(
            'px-2.5 py-0.5 rounded-full text-xs border transition-colors',
            store.statuses.includes(value)
              ? 'bg-primary text-primary-foreground border-primary'
              : 'border-border text-muted-foreground hover:border-foreground hover:text-foreground',
          )}
        >
          {label}
        </button>
      {/each}
    </div>
    {#if store.hasFilters}
      <button onclick={() => store.clearFilters()} class="text-xs text-muted-foreground hover:text-foreground ml-auto self-center">
        Clear all
      </button>
    {/if}
  </div>
{/if}

<div class="flex-1 overflow-auto">
  <div class="p-6 space-y-5">
    <div>
      <h2 class="text-2xl font-bold tracking-tight">My Library</h2>
      <p class="text-sm text-muted-foreground mt-0.5">
        {store.data ? `${store.data.total} saved ${store.data.total === 1 ? 'title' : 'titles'}` : ' '}
      </p>
    </div>

    {#if store.loading}
      <MangaGridSkeleton />
    {/if}

    {#if store.data}
      {#if store.data.items.length === 0}
        <p class="text-muted-foreground text-sm py-16 text-center">
          {store.search || store.hasFilters ? 'No results.' : 'Library is empty — discover manga to add some.'}
        </p>
      {/if}

      <div class="grid grid-cols-[repeat(auto-fill,minmax(160px,1fr))] gap-5">
        {#each store.data.items as m (m.id)}
          <MangaCard
            manga={m}
            onClick={() => router.navigate(ROUTES.title(m.id))}
            onRemove={(deleteFiles) => store.handleRemove(m.id, deleteFiles)}
            isRemoving={store.removingId === m.id}
            syncMessage={store.syncMessages[m.id]}
          />
        {/each}
      </div>

      {#if store.totalPages > 1}
        <div class="flex items-center justify-center gap-3 pt-2">
          <Button variant="outline" size="sm" disabled={store.page === 1} onclick={() => store.setPage((p) => p - 1)}>
            Prev
          </Button>
          <span class="text-sm text-muted-foreground">{store.page} / {store.totalPages}</span>
          <Button variant="outline" size="sm" disabled={store.page >= store.totalPages} onclick={() => store.setPage((p) => p + 1)}>
            Next
          </Button>
        </div>
      {/if}
    {/if}
  </div>
</div>

<button
  onclick={() => router.navigate(ROUTES.discover)}
  title="Discover manga"
  class="fixed bottom-6 right-6 w-12 h-12 rounded-full bg-primary text-primary-foreground flex items-center justify-center shadow-xl hover:opacity-90 transition-opacity z-10"
>
  <Plus class="w-5 h-5" />
</button>
