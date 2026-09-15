<script lang="ts">
  let {
    value,
    min,
    max,
    onChange,
  }: { value: number; min: number; max: number; onChange: (n: number) => void } = $props()
</script>

<div class="flex items-center gap-2">
  <button
    type="button"
    class="w-7 h-7 rounded-md bg-muted text-sm flex items-center justify-center hover:bg-accent disabled:opacity-40"
    disabled={value <= min}
    onclick={() => onChange(value - 1)}
  >−</button>
  <span class="w-5 text-center text-sm font-semibold tabular-nums">{value}</span>
  <button
    type="button"
    class="w-7 h-7 rounded-md bg-muted text-sm flex items-center justify-center hover:bg-accent disabled:opacity-40"
    disabled={value >= max}
    onclick={() => onChange(value + 1)}
  >+</button>
</div>
