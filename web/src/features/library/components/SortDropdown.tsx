import { useState, useRef, useEffect } from 'react'
import { ChevronDown } from 'lucide-react'
import { Button } from '@/components/ui/button'
import type { SortOption } from '../hooks/useLibrary'

const SORT_LABELS: Record<SortOption, string> = {
  recent: 'Recently Added',
  title_asc: 'A → Z',
  title_desc: 'Z → A',
  year: 'Year',
}

interface Props {
  value: SortOption
  onChange: (v: SortOption) => void
}

export function SortDropdown({ value, onChange }: Props) {
  const [open, setOpen] = useState(false)
  const ref = useRef<HTMLDivElement>(null)

  useEffect(() => {
    if (!open) return
    function onMouseDown(e: MouseEvent) {
      if (!ref.current?.contains(e.target as Node)) setOpen(false)
    }
    document.addEventListener('mousedown', onMouseDown)
    return () => document.removeEventListener('mousedown', onMouseDown)
  }, [open])

  return (
    <div ref={ref} className="relative">
      <Button
        variant="outline"
        size="sm"
        className="gap-1.5 text-xs"
        onClick={() => setOpen((v) => !v)}
      >
        <span className="text-muted-foreground">Sort by:</span>
        {SORT_LABELS[value]}
        <ChevronDown className={`w-3 h-3 text-muted-foreground transition-transform ${open ? 'rotate-180' : ''}`} />
      </Button>
      {open && (
        <div className="absolute right-0 top-full mt-1 w-44 rounded-md border border-border bg-background shadow-md z-20">
          {(Object.keys(SORT_LABELS) as SortOption[]).map((opt) => (
            <button
              key={opt}
              className={`w-full text-left px-3 py-2 text-sm transition-colors first:rounded-t-md last:rounded-b-md hover:bg-muted ${
                value === opt ? 'font-medium text-foreground' : 'text-muted-foreground'
              }`}
              onClick={() => { onChange(opt); setOpen(false) }}
            >
              {SORT_LABELS[opt]}
            </button>
          ))}
        </div>
      )}
    </div>
  )
}
