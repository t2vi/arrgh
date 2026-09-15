<script lang="ts">
  import { Check, Plus } from '@lucide/svelte'
  import { api, type SearchResult } from '../../lib/api'
  import Button from '../../lib/components/ui/Button.svelte'
  import Badge from '../../lib/components/ui/Badge.svelte'
  import ContentTypePill from '../../lib/components/ContentTypePill.svelte'

  let {
    result,
    inLibrary,
    addingId,
    libraryId,
    onAdd,
    onView,
  }: {
    result: SearchResult
    inLibrary: boolean
    addingId: string | null
    libraryId?: string
    onAdd: () => void
    onView: (id: string) => void
  } = $props()

  const coverSrc = $derived(result.cover_url ? api.proxyImageUrl(result.cover_url) : null)
  const isAdding = $derived(addingId === result.mangaupdates_id)
</script>

<div class="flex gap-3 rounded-lg border border-border bg-card p-3">
  {#if coverSrc}
    <img
      src={coverSrc}
      alt=""
      class="w-14 shrink-0 rounded aspect-[2/3] object-cover bg-muted"
      onerror={(e) => ((e.target as HTMLImageElement).style.visibility = 'hidden')}
    />
  {:else}
    <div class="w-14 shrink-0 rounded aspect-[2/3] bg-muted animate-pulse"></div>
  {/if}

  <div class="flex-1 min-w-0 space-y-1.5">
    <p class="font-medium text-sm leading-tight line-clamp-2">{result.title}</p>

    <div class="flex gap-1.5 flex-wrap items-center">
      <ContentTypePill type={result.content_type} />
      {#if result.is_explicit}
        <span class="inline-flex items-center px-1.5 py-px rounded text-[10px] font-bold bg-red-500/15 text-red-400">18+</span>
      {/if}
      {#if result.status && result.status !== 'unknown'}
        <Badge variant="secondary" class="capitalize text-xs">{result.status}</Badge>
      {/if}
      {#if result.author}
        <span class="text-xs text-muted-foreground">{result.author}</span>
      {/if}
      {#if result.year}
        <span class="text-xs text-muted-foreground">{result.year}</span>
      {/if}
    </div>

    {#if result.description}
      <p class="text-xs text-muted-foreground line-clamp-3">{result.description}</p>
    {:else}
      <div class="space-y-1 pt-0.5">
        <div class="h-2.5 w-full bg-muted rounded animate-pulse"></div>
        <div class="h-2.5 w-4/5 bg-muted rounded animate-pulse"></div>
      </div>
    {/if}
  </div>

  <div class="shrink-0 flex items-start pt-0.5">
    {#if inLibrary}
      <Button size="sm" variant="secondary" onclick={() => libraryId && onView(libraryId)} class="gap-1">
        <Check class="w-3 h-3" />
        In Library
      </Button>
    {:else}
      <Button size="sm" onclick={onAdd} disabled={isAdding} class="gap-1">
        {#if isAdding}
          …
        {:else}
          <Plus class="w-3 h-3" /> Add
        {/if}
      </Button>
    {/if}
  </div>
</div>
