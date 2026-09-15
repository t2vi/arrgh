<script lang="ts">
  import type { Snippet } from 'svelte'
  let { label, hint, children }: { label: string; hint: string; children: Snippet } = $props()
</script>

<div class="flex items-center justify-between gap-4">
  <div class="min-w-0">
    <p class="text-sm font-medium">{label}</p>
    <p class="text-xs text-muted-foreground">{hint}</p>
  </div>
  <div class="shrink-0">{@render children()}</div>
</div>
