import { useState } from 'react'
import { ZoomIn } from 'lucide-react'
import { Button } from '@/components/ui/button'

const LEVELS = [50, 75, 100, 125, 150] as const
type ZoomLevel = typeof LEVELS[number]
const STORAGE_KEY = 'reader-image-zoom'

export function useImageZoom() {
  const [zoom, setZoom] = useState<ZoomLevel>(() => {
    const stored = localStorage.getItem(STORAGE_KEY)
    const parsed = stored ? parseInt(stored, 10) : NaN
    return (LEVELS as readonly number[]).includes(parsed) ? (parsed as ZoomLevel) : 100
  })

  function apply(z: ZoomLevel) {
    setZoom(z)
    localStorage.setItem(STORAGE_KEY, String(z))
  }

  return { zoom, apply }
}

interface Props {
  zoom: ZoomLevel
  onApply: (zoom: ZoomLevel) => void
}

export function ZoomControl({ zoom, onApply }: Props) {
  const [open, setOpen] = useState(false)

  return (
    <div className="relative">
      <Button
        variant="ghost"
        size="icon"
        onClick={() => setOpen((v) => !v)}
        title="Zoom"
      >
        <ZoomIn className="w-4 h-4" />
      </Button>
      {open && (
        <>
          <div className="fixed inset-0 z-10" onClick={() => setOpen(false)} />
          <div className="absolute right-0 top-full mt-1 z-20 flex gap-1 p-2 rounded-lg bg-card border border-border shadow-lg">
            {LEVELS.map((l) => (
              <button
                key={l}
                type="button"
                onClick={() => { onApply(l); setOpen(false) }}
                className={`px-2.5 py-1 rounded-md text-xs font-medium transition-colors ${
                  zoom === l
                    ? 'bg-primary text-primary-foreground'
                    : 'hover:bg-muted text-muted-foreground hover:text-foreground'
                }`}
              >
                {l}%
              </button>
            ))}
          </div>
        </>
      )}
    </div>
  )
}
