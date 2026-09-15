<script lang="ts">
  import Skeleton from '../../lib/components/ui/Skeleton.svelte'
</script>

<div class="space-y-8 pb-10">
  <Skeleton class="h-72 w-full" />
  <div class="px-6 space-y-4">
    <Skeleton class="h-6 w-32" />
    <div class="flex gap-4">
      {#each Array.from({ length: 5 }) as _, i (i)}
        <div class="shrink-0 w-36 space-y-2">
          <Skeleton class="w-full aspect-[2/3] rounded-xl" />
          <Skeleton class="h-3.5 w-3/4" />
          <Skeleton class="h-3 w-1/2" />
        </div>
      {/each}
    </div>
  </div>
  <div class="px-6 space-y-4">
    <Skeleton class="h-6 w-44" />
    <div class="grid grid-cols-3 gap-3">
      {#each Array.from({ length: 3 }) as _, i (i)}
        <Skeleton class="h-20 rounded-lg" />
      {/each}
    </div>
  </div>
  <div class="px-6 space-y-4">
    <Skeleton class="h-6 w-36" />
    <div class="grid grid-cols-2 gap-4">
      <Skeleton class="aspect-[16/10] rounded-xl" />
      <Skeleton class="aspect-[16/10] rounded-xl" />
    </div>
  </div>
</div>
