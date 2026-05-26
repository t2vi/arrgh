import { useState } from 'react'
import { Button } from '@/components/ui/button'

const SIZES = [14, 16, 18, 21] as const
const STORAGE_KEY = 'reader-font-size'

export function useNovelFontSize() {
  const [size, setSize] = useState<number>(() => {
    const stored = localStorage.getItem(STORAGE_KEY)
    const parsed = stored ? parseInt(stored, 10) : NaN
    return SIZES.includes(parsed as typeof SIZES[number]) ? parsed : 16
  })

  function apply(s: number) {
    setSize(s)
    localStorage.setItem(STORAGE_KEY, String(s))
  }

  return { size, apply }
}

interface Props {
  size: number
  onApply: (size: number) => void
}

export function FontSizeControl({ size, onApply }: Props) {
  const [open, setOpen] = useState(false)

  return (
    <div className="relative">
      <Button
        variant="ghost"
        size="icon"
        onClick={() => setOpen((v) => !v)}
        title="Font size"
        className="font-semibold text-sm"
      >
        Aa
      </Button>
      {open && (
        <>
          <div className="fixed inset-0 z-10" onClick={() => setOpen(false)} />
          <div className="absolute right-0 top-full mt-1 z-20 flex gap-1 p-2 rounded-lg bg-card border border-border shadow-lg">
            {SIZES.map((s) => (
              <button
                key={s}
                type="button"
                onClick={() => { onApply(s); setOpen(false) }}
                className={`px-2.5 py-1 rounded-md text-xs font-medium transition-colors ${
                  size === s
                    ? 'bg-primary text-primary-foreground'
                    : 'hover:bg-muted text-muted-foreground hover:text-foreground'
                }`}
              >
                {s}
              </button>
            ))}
          </div>
        </>
      )}
    </div>
  )
}
