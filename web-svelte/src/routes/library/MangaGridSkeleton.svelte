<script lang="ts">
  import Skeleton from '../../lib/components/ui/Skeleton.svelte'
</script>

<div class="grid grid-cols-[repeat(auto-fill,minmax(160px,1fr))] gap-5">
  {#each Array.from({ length: 12 }) as _, i (i)}
    <div>
      <Skeleton class="w-full aspect-[2/3] rounded-xl" />
      <Skeleton class="h-3.5 mt-2.5 w-3/4 rounded" />
      <Skeleton class="h-3 mt-1.5 w-1/2 rounded" />
    </div>
  {/each}
</div>
