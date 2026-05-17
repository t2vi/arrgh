import { cn } from '@/lib/utils'

export function Toggle({ value, onChange }: { value: boolean; onChange: (v: boolean) => void }) {
  return (
    <button
      type="button"
      role="switch"
      aria-checked={value}
      className={cn('relative w-10 h-[22px] rounded-full transition-colors', value ? 'bg-primary' : 'bg-muted')}
      onClick={() => onChange(!value)}
    >
      <span className={cn(
        'absolute top-[3px] w-4 h-4 rounded-full bg-white shadow transition-transform',
        value ? 'translate-x-5' : 'translate-x-[3px]',
      )} />
    </button>
  )
}
