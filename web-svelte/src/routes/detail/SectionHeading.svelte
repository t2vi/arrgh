<script lang="ts">
  import type { Snippet } from 'svelte'
  let { children }: { children: Snippet } = $props()
</script>

<h2 class="text-xs font-semibold uppercase tracking-wider text-muted-foreground flex items-center gap-2">
  <span class="w-0.5 h-3.5 bg-primary rounded-full inline-block shrink-0"></span>
  {@render children()}
</h2>
