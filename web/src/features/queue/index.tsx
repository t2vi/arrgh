import { useNavigate } from 'react-router-dom'
import { ChevronRight, Loader2, Trash2 } from 'lucide-react'
import { Button } from '@/components/ui/button'
import { ScrollArea } from '@/components/ui/scroll-area'
import { useQueue } from './hooks/useQueue'
import { QueueRow } from './components/QueueRow'

export default function Queue() {
  const navigate = useNavigate()
  const h = useQueue()

  return (
    <div className="flex flex-col h-full">
      <header className="flex items-center gap-2 px-4 py-3 border-b border-border bg-card shrink-0">
        <Button variant="ghost" size="icon" onClick={() => navigate(-1)}>
          <ChevronRight className="w-4 h-4 rotate-180" />
        </Button>
        <h1 className="text-base font-semibold flex-1">Download Queue</h1>
        {h.canClear && (
          <Button
            variant="ghost"
            size="sm"
            className="gap-1.5 text-muted-foreground hover:text-foreground text-xs"
            onClick={h.handleClearCompleted}
            disabled={h.clearingCompleted}
          >
            {h.clearingCompleted
              ? <Loader2 className="w-3 h-3 animate-spin" />
              : <Trash2 className="w-3 h-3" />}
            Clear completed
          </Button>
        )}
      </header>

      <ScrollArea className="flex-1">
        <div className="p-4 space-y-1">
          {h.loading && <p className="text-muted-foreground text-sm">Loading…</p>}
          {!h.loading && h.data?.length === 0 && (
            <p className="text-muted-foreground text-sm">Queue is empty.</p>
          )}
          {h.data?.map((item) => (
            <QueueRow
              key={item.id}
              item={item}
              onRemove={() => h.handleRemove(item.id)}
            />
          ))}
        </div>
      </ScrollArea>
    </div>
  )
}
