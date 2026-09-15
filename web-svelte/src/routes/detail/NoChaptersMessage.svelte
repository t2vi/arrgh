<script lang="ts">
  import { RefreshCw } from '@lucide/svelte'

  let {
    hasSyncWarnings,
    isSyncing,
    isPending,
    isRemoteSource,
    onSync,
  }: {
    hasSyncWarnings: boolean
    isSyncing: boolean
    isPending: boolean
    isRemoteSource: boolean
    onSync: () => void
  } = $props()
</script>

{#if isSyncing || isPending}
  <p class="text-sm text-muted-foreground py-4 flex items-center gap-2">
    <RefreshCw class="w-3.5 h-3.5 animate-spin" /> Fetching chapters…
  </p>
{:else if hasSyncWarnings}
  <p class="text-sm text-muted-foreground py-4">
    No sources found for this title — none of the available plugins carry it.
    {#if isRemoteSource}
      <button onclick={onSync} class="underline hover:text-foreground">Retry.</button>
    {/if}
  </p>
{:else}
  <p class="text-sm text-muted-foreground py-4">
    No chapters.{#if isRemoteSource}<button onclick={onSync} class="underline hover:text-foreground ml-1">Sync.</button>{/if}
  </p>
{/if}
