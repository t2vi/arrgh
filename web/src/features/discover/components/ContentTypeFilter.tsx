import { cn } from '@/lib/utils'

const CONTENT_TYPES = [
  { value: 'manga',     label: 'Manga'    },
  { value: 'manhwa',   label: 'Manhwa'   },
  { value: 'manhua',   label: 'Manhua'   },
  { value: 'one-shot', label: 'One-shot' },
  { value: 'novel',    label: 'Novel'    },
] as const

export function ContentTypeFilter({
  value,
  onChange,
  availableTypes,
}: {
  value: string | undefined
  onChange: (v: string | undefined) => void
  availableTypes: Set<string>
}) {
  const visible = CONTENT_TYPES.filter((ct) => availableTypes.has(ct.value))
  if (visible.length === 0) return null

  return (
    <div className="flex gap-1 flex-wrap">
      <button
        type="button"
        onClick={() => onChange(undefined)}
        className={cn(
          'px-2.5 py-0.5 rounded-full text-xs font-medium transition-colors border',
          value === undefined
            ? 'bg-primary text-primary-foreground border-primary'
            : 'bg-muted text-muted-foreground border-transparent hover:border-border hover:text-foreground',
        )}
      >
        All
      </button>
      {visible.map((ct) => (
        <button
          key={ct.label}
          type="button"
          onClick={() => onChange(ct.value)}
          className={cn(
            'px-2.5 py-0.5 rounded-full text-xs font-medium transition-colors border',
            value === ct.value
              ? 'bg-primary text-primary-foreground border-primary'
              : 'bg-muted text-muted-foreground border-transparent hover:border-border hover:text-foreground',
          )}
        >
          {ct.label}
        </button>
      ))}
    </div>
  )
}
