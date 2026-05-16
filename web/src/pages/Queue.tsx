import { useEffect, useRef } from 'react'
import { useQuery, useMutation, useQueryClient } from '@tanstack/react-query'
import { useNavigate } from 'react-router-dom'
import { ChevronRight, X, Loader2, Check, AlertCircle, Clock, Trash2 } from 'lucide-react'
import { api, type QueueItem } from '@/api'
import { Button } from '@/components/ui/button'
import { Badge } from '@/components/ui/badge'
import { ScrollArea } from '@/components/ui/scroll-area'
import { cn } from '@/lib/utils'

const STATUS: Record<string, { icon: React.ReactNode; badge: 'default' | 'secondary' | 'destructive' | 'outline' }> = {
  pending:     { icon: <Clock className="w-3.5 h-3.5" />,                badge: 'secondary'  },
  downloading: { icon: <Loader2 className="w-3.5 h-3.5 animate-spin" />, badge: 'default'    },
  done:        { icon: <Check className="w-3.5 h-3.5" />,                badge: 'outline'    },
  error:       { icon: <AlertCircle className="w-3.5 h-3.5" />,          badge: 'destructive'},
  cancelled:   { icon: <X className="w-3.5 h-3.5" />,                    badge: 'outline'    },
}

const STATUS_ORDER: Record<string, number> = {
  downloading: 0, pending: 1, done: 2, error: 3, cancelled: 4,
}

function isActive(item: QueueItem) {
  return item.status === 'pending' || item.status === 'downloading'
}

export default function Queue() {
  const navigate = useNavigate()
  const qc = useQueryClient()
  const hadActiveRef = useRef(false)

  const { data: raw, isLoading } = useQuery({
    queryKey: ['queue'],
    queryFn: api.getQueue,
    refetchInterval: 2000,
  })

  const data = raw?.slice().sort(
    (a, b) => (STATUS_ORDER[a.status] ?? 9) - (STATUS_ORDER[b.status] ?? 9)
  )

  const remove = useMutation({
    mutationFn: api.removeFromQueue,
    onSuccess: () => qc.invalidateQueries({ queryKey: ['queue'] }),
  })

  const clearCompleted = useMutation({
    mutationFn: api.clearCompletedQueue,
    onSuccess: () => qc.invalidateQueries({ queryKey: ['queue'] }),
  })

  // Auto-clear done/cancelled/error once all active work drains
  useEffect(() => {
    if (!data) return
    const hasActive = data.some(isActive)
    const hasFinished = data.some(i => i.status === 'done' || i.status === 'cancelled')
    if (hadActiveRef.current && !hasActive && hasFinished) {
      clearCompleted.mutate()
    }
    hadActiveRef.current = hasActive
  }, [data])

  const hasCompletedItems = data?.some(i => !isActive(i) && i.status !== 'error') ?? false
  const hasErrorItems = data?.some(i => i.status === 'error') ?? false
  const canClear = hasCompletedItems || hasErrorItems

  return (
    <div className="flex flex-col h-full">
      <header className="flex items-center gap-2 px-4 py-3 border-b border-border bg-card shrink-0">
        <Button variant="ghost" size="icon" onClick={() => navigate(-1)}>
          <ChevronRight className="w-4 h-4 rotate-180" />
        </Button>
        <h1 className="text-base font-semibold flex-1">Download Queue</h1>
        {canClear && (
          <Button
            variant="ghost"
            size="sm"
            className="gap-1.5 text-muted-foreground hover:text-foreground text-xs"
            onClick={() => clearCompleted.mutate()}
            disabled={clearCompleted.isPending}
          >
            {clearCompleted.isPending
              ? <Loader2 className="w-3 h-3 animate-spin" />
              : <Trash2 className="w-3 h-3" />}
            Clear completed
          </Button>
        )}
      </header>

      <ScrollArea className="flex-1">
        <div className="p-4 space-y-1">
          {isLoading && <p className="text-muted-foreground text-sm">Loading…</p>}
          {!isLoading && data?.length === 0 && (
            <p className="text-muted-foreground text-sm">Queue is empty.</p>
          )}
          {data?.map((item) => (
            <QueueRow
              key={item.id}
              item={item}
              onRemove={() => remove.mutate(item.id)}
            />
          ))}
        </div>
      </ScrollArea>
    </div>
  )
}

function QueueRow({ item, onRemove }: { item: QueueItem; onRemove: () => void }) {
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
