import { Clock, Loader2, Check, AlertCircle, X } from 'lucide-react'
import { type QueueItem } from '@/api'
import { Badge } from '@/components/ui/badge'
import { Button } from '@/components/ui/button'
import { cn } from '@/lib/utils'

const STATUS: Record<string, { icon: React.ReactNode; badge: 'default' | 'secondary' | 'destructive' | 'outline' }> = {
  pending:     { icon: <Clock className="w-3.5 h-3.5" />,                badge: 'secondary'   },
  downloading: { icon: <Loader2 className="w-3.5 h-3.5 animate-spin" />, badge: 'default'     },
  done:        { icon: <Check className="w-3.5 h-3.5" />,                badge: 'outline'     },
  error:       { icon: <AlertCircle className="w-3.5 h-3.5" />,          badge: 'destructive' },
  cancelled:   { icon: <X className="w-3.5 h-3.5" />,                    badge: 'outline'     },
}

export function QueueRow({ item, onRemove }: { item: QueueItem; onRemove: () => void }) {
  const s = STATUS[item.status] ?? STATUS.pending

  return (
    <div className="flex items-center gap-3 py-3 border-b border-border last:border-0">
      <span className={cn(
        'shrink-0',
        item.status === 'error' ? 'text-destructive' :
        item.status === 'done' ? 'text-muted-foreground' : 'text-foreground',
      )}>
        {s.icon}
      </span>

      <div className="flex-1 min-w-0">
        <p className="text-sm font-medium truncate">{item.manga_title}</p>
        <p className="text-xs text-muted-foreground">Ch. {item.chapter_num}</p>
        {item.error && (
          <p className="text-xs text-destructive mt-0.5 line-clamp-1">{item.error}</p>
        )}
      </div>

      <Badge variant={s.badge} className="shrink-0 capitalize">{item.status}</Badge>

      {item.status !== 'downloading' && (
        <Button variant="ghost" size="icon" className="shrink-0 h-7 w-7" onClick={onRemove}>
          <X className="w-3 h-3" />
        </Button>
      )}
    </div>
  )
}
