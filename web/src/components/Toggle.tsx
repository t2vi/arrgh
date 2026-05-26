import { cn } from '@/lib/utils'

export function Toggle({ value, onChange }: { value: boolean; onChange: (v: boolean) => void }) {
  return (
    <button
      type="button"
      role="switch"
      aria-checked={value}
      className={cn('relative w-11 h-6 rounded-full transition-colors overflow-hidden', value ? 'bg-primary' : 'bg-muted')}
      onClick={() => onChange(!value)}
    >
      <span className={cn(
        'absolute top-[3px] left-0 w-[18px] h-[18px] rounded-full bg-white shadow transition-transform',
        value ? 'translate-x-[23px]' : 'translate-x-[3px]',
      )} />
    </button>
  )
}
