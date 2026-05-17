import { cn } from '@/lib/utils'

export function SegmentedControl({ value, options, onChange }: {
  value: string
  options: { value: string; label: string }[]
  onChange: (v: string) => void
}) {
  return (
    <div className="flex rounded-lg bg-muted p-0.5 gap-0.5">
      {options.map((o) => (
        <button
          key={o.value}
          type="button"
          className={cn(
            'px-3 py-1 rounded-md text-xs font-semibold transition-colors',
            value === o.value ? 'bg-card text-foreground shadow-sm' : 'text-muted-foreground hover:text-foreground',
          )}
          onClick={() => onChange(o.value)}
        >
          {o.label}
        </button>
      ))}
    </div>
  )
}
