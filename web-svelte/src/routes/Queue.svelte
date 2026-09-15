<script lang="ts">
  import { ChevronRight, LoaderCircle, Trash2 } from '@lucide/svelte'
  import { QueueStore } from '../lib/queue.svelte'
  import Button from '../lib/components/ui/Button.svelte'
  import QueueRow from './queue/QueueRow.svelte'

  let _params: Record<string, string> = $props()

  const store = new QueueStore()
</script>

<div class="flex flex-col h-full">
  <header class="flex items-center gap-2 px-4 py-3 border-b border-border bg-card shrink-0">
    <Button variant="ghost" size="icon" onclick={() => window.history.back()}>
      <ChevronRight class="w-4 h-4 rotate-180" />
    </Button>
    <h1 class="text-base font-semibold flex-1">Download Queue</h1>
    {#if store.canClear}
      <Button
        variant="ghost"
        size="sm"
        class="gap-1.5 text-muted-foreground hover:text-foreground text-xs"
        onclick={store.handleClearCompleted}
        disabled={store.clearingCompleted}
      >
        {#if store.clearingCompleted}
          <LoaderCircle class="w-3 h-3 animate-spin" />
        {:else}
          <Trash2 class="w-3 h-3" />
        {/if}
        Clear completed
      </Button>
    {/if}
  </header>

  <div class="flex-1 overflow-y-auto">
    <div class="p-4 space-y-1">
      {#if store.loading}
        <p class="text-muted-foreground text-sm">Loading…</p>
      {/if}
      {#if !store.loading && store.data?.length === 0}
        <p class="text-muted-foreground text-sm">Queue is empty.</p>
      {/if}
      {#each store.data ?? [] as item (item.id)}
        <QueueRow {item} onRemove={() => store.handleRemove(item.id)} />
      {/each}
    </div>
  </div>
</div>
