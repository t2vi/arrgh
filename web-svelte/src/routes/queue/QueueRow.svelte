<script lang="ts">
  import { Clock, LoaderCircle, Check, CircleAlert, X } from '@lucide/svelte'
  import type { QueueItem } from '../../lib/api'
  import Badge from '../../lib/components/ui/Badge.svelte'
  import Button from '../../lib/components/ui/Button.svelte'
  import { cn } from '../../lib/utils'

  const STATUS: Record<string, { icon: typeof Clock; badge: 'default' | 'secondary' | 'destructive' | 'outline' }> = {
    pending:     { icon: Clock,        badge: 'secondary'   },
    downloading: { icon: LoaderCircle, badge: 'default'     },
    done:        { icon: Check,        badge: 'outline'     },
    error:       { icon: CircleAlert,  badge: 'destructive' },
    cancelled:   { icon: X,            badge: 'outline'     },
  }

  let { item, onRemove }: { item: QueueItem; onRemove: () => void } = $props()

  const s = $derived(STATUS[item.status] ?? STATUS.pending)
  const pct = $derived(item.pages_total > 0 ? Math.round((item.pages_downloaded / item.pages_total) * 100) : 0)
</script>

<div class="flex items-center gap-3 py-3 border-b border-border last:border-0">
  <span class={cn(
    'shrink-0',
    item.status === 'error' ? 'text-destructive' :
    item.status === 'done' ? 'text-muted-foreground' : 'text-foreground',
  )}>
    <s.icon class={cn('w-3.5 h-3.5', item.status === 'downloading' && 'animate-spin')} />
  </span>

  <div class="flex-1 min-w-0">
    <p class="text-sm font-medium truncate">{item.manga_title}</p>
    <p class="text-xs text-muted-foreground">Ch. {item.chapter_num}</p>
    {#if item.status === 'downloading' && item.pages_total > 0}
      <div class="mt-1.5 flex items-center gap-2">
        <div class="flex-1 h-1 bg-muted rounded-full overflow-hidden">
          <div class="h-full bg-primary transition-all duration-300" style={`width: ${pct}%`}></div>
        </div>
        <span class="text-xs text-muted-foreground tabular-nums shrink-0">{pct}%</span>
      </div>
    {/if}
    {#if item.error}
      <p class="text-xs text-destructive mt-0.5 line-clamp-1">{item.error}</p>
    {/if}
  </div>

  <Badge variant={s.badge} class="shrink-0 capitalize">{item.status}</Badge>

  {#if item.status !== 'downloading'}
    <Button variant="ghost" size="icon" class="shrink-0 h-7 w-7" onclick={onRemove}>
      <X class="w-3 h-3" />
    </Button>
  {/if}
</div>
