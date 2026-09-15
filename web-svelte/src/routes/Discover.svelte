<script lang="ts">
  import { Search, ChevronRight, AlertCircle } from '@lucide/svelte'
  import Input from '../lib/components/ui/Input.svelte'
  import Button from '../lib/components/ui/Button.svelte'
  import { router } from '../lib/router.svelte'
  import { DiscoverStore } from '../lib/discover.svelte'
  import SearchRow from './discover/SearchRow.svelte'
  import SearchProgress from './discover/SearchProgress.svelte'
  import ContentTypeFilter from './discover/ContentTypeFilter.svelte'

  let _params: Record<string, string> = $props()

  const store = new DiscoverStore()
</script>

<div class="flex flex-col h-full">
  <header class="flex items-center gap-3 px-4 py-3 border-b border-border bg-card shrink-0">
    <Button variant="ghost" size="icon" onclick={() => window.history.back()} title="Back">
      <ChevronRight class="w-4 h-4 rotate-180" />
    </Button>
    <h1 class="text-base font-semibold">Discover</h1>
  </header>

  <div class="flex-1 overflow-y-auto">
    <div class="p-4 space-y-4">
      <div class="flex gap-2">
        <div class="relative flex-1">
          <Search class="absolute left-3 top-1/2 -translate-y-1/2 w-4 h-4 text-muted-foreground" />
          <Input
            class="pl-9"
            placeholder="Search for manga, manhwa, manhua, novels, or hentai…"
            value={store.query}
            oninput={(e: Event) => (store.query = (e.currentTarget as HTMLInputElement).value)}
            onkeydown={(e: KeyboardEvent) => e.key === 'Enter' && store.submit()}
            autofocus
          />
        </div>
        <Button onclick={() => store.submit()} disabled={store.isFetching}>
          {store.isFetching ? '…' : 'Search'}
        </Button>
      </div>

      {#if store.searchError || store.addError}
        <div class="flex items-center gap-2 text-destructive text-sm rounded-lg border border-destructive/30 bg-destructive/10 px-3 py-2">
          <AlertCircle class="w-4 h-4 shrink-0" />
          {store.addError ?? store.searchError}
        </div>
      {/if}

      {#if store.isFetching || store.showProgress}
        <SearchProgress completedSources={store.showProgress ? store.completedSources : undefined} />
      {/if}

      {#if store.data && !store.isFetching && !store.showProgress}
        <div class="space-y-3">
          <ContentTypeFilter
            value={store.contentTypeFilter}
            onChange={(v) => store.setContentTypeFilter(v)}
            availableTypes={store.availableTypes}
          />

          {#if (store.filteredData?.length ?? 0) === 0}
            <p class="text-muted-foreground text-sm">No results.</p>
          {/if}

          {#each store.filteredData ?? [] as r (`${r.source}:${r.mangaupdates_id}`)}
            {@const inLibraryNow = r.in_library || store.added.has(r.mangaupdates_id)}
            {@const libraryId = r.library_id ?? store.added.get(r.mangaupdates_id)}
            <SearchRow
              result={r}
              inLibrary={inLibraryNow}
              addingId={store.addingId}
              libraryId={libraryId ?? undefined}
              onAdd={() => store.handleAdd(r)}
              onView={(id) => router.navigate(`/title/${id}`)}
            />
          {/each}
        </div>
      {/if}
    </div>
  </div>
</div>
