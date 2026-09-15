<script lang="ts">
  import type { Snippet } from 'svelte'
  let { label, description, children }: { label: string; description: string; children: Snippet } = $props()
</script>

<div class="rounded-lg bg-card border border-border p-4 space-y-3">
  <p class="text-[10px] font-semibold uppercase tracking-wider text-muted-foreground">{label}</p>
  {@render children()}
  <p class="text-[10px] text-muted-foreground leading-relaxed">{description}</p>
</div>
