<script lang="ts">
  import Skeleton from '../../lib/components/ui/Skeleton.svelte'
</script>

<div class="space-y-1.5">
  {#each Array.from({ length: 5 }) as _, i (i)}
    <div class="flex items-center gap-3 px-3 py-2.5 rounded-lg border border-border bg-card">
      <Skeleton class="w-11 h-7 rounded-md shrink-0" />
      <div class="flex-1 space-y-1.5">
        <Skeleton class="h-4 w-1/2" />
        <Skeleton class="h-3 w-1/3" />
      </div>
      <Skeleton class="w-7 h-7 rounded-md shrink-0" />
    </div>
  {/each}
</div>
