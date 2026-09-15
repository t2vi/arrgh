<script lang="ts">
  let { value }: { value: number } = $props()
</script>

<div class="h-[3px] w-full bg-muted shrink-0">
  <div class="h-full bg-primary transition-[width] duration-150" style={`width: ${Math.min(1, Math.max(0, value)) * 100}%`}></div>
</div>
