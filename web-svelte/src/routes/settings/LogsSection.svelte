<script lang="ts">
  import { api, type LogEntry } from '../../lib/api'
  import { cn } from '../../lib/utils'

  const LEVELS = ['ERROR', 'WARN', 'INFO', 'DEBUG'] as const
  type Level = typeof LEVELS[number]

  const LEVEL_COLORS: Record<Level, string> = {
    ERROR: 'text-red-400',
    WARN: 'text-yellow-400',
    INFO: 'text-blue-300',
    DEBUG: 'text-muted-foreground',
  }

  let entries = $state<LogEntry[]>([])
  let level = $state<Level>('INFO')
  let filter = $state<Level | 'ALL'>('ALL')
  let loading = $state(true)

  api.getLogLevel().then((r) => (level = r.level as Level)).catch(() => {})

  $effect(() => {
    let alive = true
    function poll() {
      api.getLogs(500)
        .then((data) => { if (alive) { entries = data; loading = false } })
        .catch(() => { if (alive) loading = false })
    }
    poll()
    const id = setInterval(poll, 3000)
    return () => { alive = false; clearInterval(id) }
  })

  async function handleLevelChange(l: Level) {
    level = l
    await api.setLogLevel(l).catch(() => {})
  }

  const visible = $derived(filter === 'ALL' ? entries : entries.filter((e) => e.level === filter))
</script>

<div class="space-y-4">
  <div class="flex flex-wrap items-end gap-4">
    <div>
      <p class="text-xs font-semibold uppercase tracking-widest text-muted-foreground mb-1.5">Capture Level</p>
      <div class="flex rounded-lg bg-muted p-0.5 gap-0.5">
        {#each LEVELS as l (l)}
          <button
            type="button"
            onclick={() => handleLevelChange(l)}
            class={cn(
              'px-2.5 py-1 rounded-md text-xs font-semibold transition-colors',
              level === l ? 'bg-card text-foreground shadow-sm' : 'text-muted-foreground hover:text-foreground',
            )}
          >
            {l}
          </button>
        {/each}
      </div>
    </div>

    <div>
      <p class="text-xs font-semibold uppercase tracking-widest text-muted-foreground mb-1.5">Show</p>
      <div class="flex rounded-lg bg-muted p-0.5 gap-0.5">
        {#each ['ALL', ...LEVELS] as l (l)}
          <button
            type="button"
            onclick={() => (filter = l as Level | 'ALL')}
            class={cn(
              'px-2.5 py-1 rounded-md text-xs font-semibold transition-colors',
              filter === l ? 'bg-card text-foreground shadow-sm' : 'text-muted-foreground hover:text-foreground',
            )}
          >
            {l}
          </button>
        {/each}
      </div>
    </div>
  </div>

  <div class="rounded-lg bg-muted border border-border overflow-auto h-[28rem] font-mono text-xs">
    {#if loading}
      <p class="p-4 text-muted-foreground">Loading…</p>
    {:else if visible.length === 0}
      <p class="p-4 text-muted-foreground">No entries.</p>
    {:else}
      <table class="w-full">
        <tbody>
          {#each visible as e, i (i)}
            <tr class="border-b border-border/30 hover:bg-card/40">
              <td class="px-3 py-0.5 text-muted-foreground whitespace-nowrap">
                {new Date(e.timestamp).toLocaleTimeString()}
              </td>
              <td class={cn('px-2 py-0.5 font-bold w-14 shrink-0', LEVEL_COLORS[e.level as Level] ?? '')}>
                {e.level}
              </td>
              <td class="px-2 py-0.5 text-muted-foreground whitespace-nowrap max-w-[10rem] truncate">
                {e.target.replace('arrgh_server::', '')}
              </td>
              <td class="px-2 py-0.5 text-foreground break-all">
                {e.message}
              </td>
            </tr>
          {/each}
        </tbody>
      </table>
    {/if}
  </div>

  <p class="text-xs text-muted-foreground">
    {visible.length} entries · refreshes every 3s · set <span class="font-mono">RUST_LOG</span> env var to change stdout level
  </p>
</div>
