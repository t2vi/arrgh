<script lang="ts">
  import { cn } from '../utils'
  let {
    value,
    onChange,
    label = 'Toggle',
  }: { value: boolean; onChange: (v: boolean) => void; label?: string } = $props()
</script>

<button
  type="button"
  role="switch"
  aria-checked={value}
  aria-label={label}
  class={cn('relative w-11 h-6 rounded-full transition-colors overflow-hidden', value ? 'bg-primary' : 'bg-muted')}
  onclick={() => onChange(!value)}
>
  <span
    class={cn(
      'absolute top-[3px] left-0 w-[18px] h-[18px] rounded-full bg-white shadow transition-transform',
      value ? 'translate-x-[23px]' : 'translate-x-[3px]',
    )}
  ></span>
</button>
