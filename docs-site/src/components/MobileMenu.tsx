import { useState } from 'react'
import { Menu, X } from 'lucide-react'
import { cn } from '@/lib/utils'

const NAV_ITEMS = [
  { label: 'Getting Started', href: '/arrgh/getting-started' },
  { label: 'Deploy', href: '/arrgh/deploy/docker-compose' },
  { label: 'Plugins', href: '/arrgh/plugins' },
  { label: 'Releases', href: '/arrgh/releases' },
  { label: 'Test Reports', href: '/test-reports', external: true },
]

export function MobileMenu({ current }: { current: string }) {
  const [open, setOpen] = useState(false)

  return (
    <div className="md:hidden">
      <button
        onClick={() => setOpen((v) => !v)}
        className="w-8 h-8 flex items-center justify-center rounded-md text-muted-foreground hover:text-foreground hover:bg-muted transition-colors"
        aria-label="Toggle menu"
      >
        {open ? <X className="w-4 h-4" /> : <Menu className="w-4 h-4" />}
      </button>

      {open && (
        <div className="absolute top-full left-0 right-0 bg-card/95 backdrop-blur border-b border-border p-4 flex flex-col gap-1 z-50">
          {NAV_ITEMS.map((item) => (
            <a
              key={item.href}
              href={item.href}
              target={item.external ? '_blank' : undefined}
              rel={item.external ? 'noopener noreferrer' : undefined}
              onClick={() => setOpen(false)}
              className={cn(
                'px-3 py-2 rounded-md text-sm transition-colors',
                current.startsWith(item.href)
                  ? 'bg-primary/15 text-primary font-medium'
                  : 'text-muted-foreground hover:text-foreground hover:bg-muted',
              )}
            >
              {item.label}
              {item.external && <span className="ml-1 text-xs opacity-60">↗</span>}
            </a>
          ))}
        </div>
      )}
    </div>
  )
}
