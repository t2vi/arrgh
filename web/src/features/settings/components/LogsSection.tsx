import { useState, useEffect } from 'react'
import { api } from '@/api'
import type { LogEntry } from '@/api'
import { cn } from '@/lib/utils'

const LEVELS = ['ERROR', 'WARN', 'INFO', 'DEBUG'] as const
type Level = typeof LEVELS[number]

const LEVEL_COLORS: Record<Level, string> = {
  ERROR: 'text-red-400',
  WARN:  'text-yellow-400',
  INFO:  'text-blue-300',
  DEBUG: 'text-muted-foreground',
}

export function LogsSection() {
  const [entries, setEntries]   = useState<LogEntry[]>([])
  const [level, setLevel]       = useState<Level>('INFO')
  const [filter, setFilter]     = useState<Level | 'ALL'>('ALL')
  const [loading, setLoading]   = useState(true)

  useEffect(() => {
    api.getLogLevel()
      .then(r => setLevel(r.level as Level))
      .catch(() => {})
  }, [])

  useEffect(() => {
    let alive = true
    function poll() {
      api.getLogs(500)
        .then(data => { if (alive) { setEntries(data); setLoading(false) } })
        .catch(() => { if (alive) setLoading(false) })
    }
    poll()
    const id = setInterval(poll, 3000)
    return () => { alive = false; clearInterval(id) }
  }, [])

  async function handleLevelChange(l: Level) {
    setLevel(l)
    await api.setLogLevel(l).catch(() => {})
  }

  const visible = filter === 'ALL' ? entries : entries.filter(e => e.level === filter)

  return (
    <div className="space-y-4">
      <div className="flex flex-wrap items-end gap-4">
        <div>
          <p className="text-xs font-semibold uppercase tracking-widest text-muted-foreground mb-1.5">
            Capture Level
          </p>
          <div className="flex rounded-lg bg-muted p-0.5 gap-0.5">
            {LEVELS.map(l => (
              <button
                key={l}
                type="button"
                onClick={() => handleLevelChange(l)}
                className={cn(
                  'px-2.5 py-1 rounded-md text-xs font-semibold transition-colors',
                  level === l
                    ? 'bg-card text-foreground shadow-sm'
                    : 'text-muted-foreground hover:text-foreground',
                )}
              >
                {l}
              </button>
            ))}
          </div>
        </div>

        <div>
          <p className="text-xs font-semibold uppercase tracking-widest text-muted-foreground mb-1.5">
            Show
          </p>
          <div className="flex rounded-lg bg-muted p-0.5 gap-0.5">
            {(['ALL', ...LEVELS] as const).map(l => (
              <button
                key={l}
                type="button"
                onClick={() => setFilter(l)}
                className={cn(
                  'px-2.5 py-1 rounded-md text-xs font-semibold transition-colors',
                  filter === l
                    ? 'bg-card text-foreground shadow-sm'
                    : 'text-muted-foreground hover:text-foreground',
                )}
              >
                {l}
              </button>
            ))}
          </div>
        </div>
      </div>

      <div className="rounded-lg bg-muted border border-border overflow-auto h-[28rem] font-mono text-xs">
        {loading ? (
          <p className="p-4 text-muted-foreground">Loading…</p>
        ) : visible.length === 0 ? (
          <p className="p-4 text-muted-foreground">No entries.</p>
        ) : (
          <table className="w-full">
            <tbody>
              {visible.map((e, i) => (
                <tr key={i} className="border-b border-border/30 hover:bg-card/40">
                  <td className="px-3 py-0.5 text-muted-foreground whitespace-nowrap">
                    {new Date(e.timestamp).toLocaleTimeString()}
                  </td>
                  <td className={cn('px-2 py-0.5 font-bold w-14 shrink-0', LEVEL_COLORS[e.level as Level] ?? '')}>
                    {e.level}
                  </td>
                  <td className="px-2 py-0.5 text-muted-foreground whitespace-nowrap max-w-[10rem] truncate">
                    {e.target.replace('arrgh_server::', '')}
                  </td>
                  <td className="px-2 py-0.5 text-foreground break-all">
                    {e.message}
                  </td>
                </tr>
              ))}
            </tbody>
          </table>
        )}
      </div>

      <p className="text-xs text-muted-foreground">
        {visible.length} entries · refreshes every 3s · set <span className="font-mono">RUST_LOG</span> env var to change stdout level
      </p>
    </div>
  )
}
