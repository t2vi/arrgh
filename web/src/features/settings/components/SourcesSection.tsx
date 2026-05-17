import { useState, useEffect, useCallback } from 'react'
import { Loader2, Globe, KeyRound, Plus, Trash2 } from 'lucide-react'
import { api, type SourceRow } from '@/api'
import { cn } from '@/lib/utils'

function SourceRowItem({ source, onToggle, onDelete }: {
  source: SourceRow
  onToggle: (id: string, enabled: boolean) => void
  onDelete: (id: string) => void
}) {
  return (
    <div className="flex items-center gap-2 px-3 py-2 rounded-lg bg-muted/50">
      <div className="flex-1 min-w-0">
        <div className="flex items-center gap-2">
          <p className="text-sm font-medium truncate">{source.name}</p>
          {source.has_api_key && (
            <KeyRound className="w-3 h-3 text-muted-foreground shrink-0" title="Uses API key" />
          )}
        </div>
        <div className="flex items-center gap-1.5 mt-0.5 flex-wrap">
          <p className="text-xs text-muted-foreground truncate">{source.base_url}</p>
          {source.content_types.map((ct) => (
            <span key={ct} className="px-1.5 py-0 rounded-full bg-card border border-border text-[10px] text-muted-foreground">
              {ct}
            </span>
          ))}
        </div>
      </div>

      <button
        type="button"
        title={source.enabled ? 'Enabled — click to disable' : 'Disabled — click to enable'}
        onClick={() => onToggle(source.id, source.enabled)}
        className={cn(
          'relative w-8 h-[18px] rounded-full transition-colors shrink-0',
          source.enabled ? 'bg-primary' : 'bg-muted border border-border',
        )}
      >
        <span className={cn(
          'absolute top-[2px] w-3.5 h-3.5 rounded-full bg-white shadow transition-transform',
          source.enabled ? 'translate-x-[14px]' : 'translate-x-[2px]',
        )} />
      </button>

      <button
        type="button"
        title="Remove source"
        onClick={() => onDelete(source.id)}
        className="w-6 h-6 flex items-center justify-center rounded-md text-muted-foreground hover:text-destructive hover:bg-destructive/10 transition-colors"
      >
        <Trash2 className="w-3.5 h-3.5" />
      </button>
    </div>
  )
}

export function SourcesSection() {
  const [sources, setSources] = useState<SourceRow[]>([])
  const [loading, setLoading] = useState(true)
  const [url, setUrl] = useState('')
  const [apiKey, setApiKey] = useState('')
  const [adding, setAdding] = useState(false)
  const [addError, setAddError] = useState('')

  const reload = useCallback(() => {
    setLoading(true)
    api.listSources()
      .then(setSources)
      .catch(() => {})
      .finally(() => setLoading(false))
  }, [])

  useEffect(() => { reload() }, [reload])

  async function handleAdd(e: React.FormEvent) {
    e.preventDefault()
    setAddError('')
    setAdding(true)
    try {
      await api.addSource(url.trim(), apiKey.trim() || undefined)
      setUrl('')
      setApiKey('')
      reload()
    } catch (err: unknown) {
      const msg = err instanceof Error ? err.message : ''
      if (msg.includes('502')) setAddError('Could not reach plugin — check the URL.')
      else setAddError('Failed to add source.')
    } finally {
      setAdding(false)
    }
  }

  async function handleToggle(id: string, enabled: boolean) {
    await api.patchSource(id, !enabled)
    reload()
  }

  async function handleDelete(id: string) {
    await api.deleteSource(id)
    reload()
  }

  return (
    <section className="space-y-4">
      <p className="text-xs text-muted-foreground">
        External source plugins extend *ARRgh with additional manga sources. Each plugin is a separate HTTP server implementing the Source Plugin Protocol.
      </p>

      {loading ? (
        <Loader2 className="w-4 h-4 animate-spin text-muted-foreground" />
      ) : sources.length === 0 ? (
        <p className="text-sm text-muted-foreground">No external sources yet.</p>
      ) : (
        <div className="space-y-1.5">
          {sources.map((s) => (
            <SourceRowItem key={s.id} source={s} onToggle={handleToggle} onDelete={handleDelete} />
          ))}
        </div>
      )}

      <form onSubmit={handleAdd} className="space-y-2 pt-2 border-t border-border">
        <p className="text-xs font-medium text-muted-foreground">Add plugin source</p>
        <div className="flex gap-2">
          <div className="relative flex-1">
            <Globe className="absolute left-2.5 top-1/2 -translate-y-1/2 w-3.5 h-3.5 text-muted-foreground pointer-events-none" />
            <input
              className="w-full pl-8 pr-3 py-1.5 rounded-md bg-muted border border-border text-sm placeholder:text-muted-foreground focus:outline-none focus:ring-1 focus:ring-ring"
              placeholder="http://localhost:4000"
              value={url}
              onChange={(e) => setUrl(e.target.value)}
              required
            />
          </div>
          <div className="relative w-40">
            <KeyRound className="absolute left-2.5 top-1/2 -translate-y-1/2 w-3.5 h-3.5 text-muted-foreground pointer-events-none" />
            <input
              className="w-full pl-8 pr-3 py-1.5 rounded-md bg-muted border border-border text-sm placeholder:text-muted-foreground focus:outline-none focus:ring-1 focus:ring-ring"
              placeholder="API key (opt.)"
              value={apiKey}
              onChange={(e) => setApiKey(e.target.value)}
            />
          </div>
          <button
            type="submit"
            disabled={adding}
            className="flex items-center gap-1 px-3 py-1.5 rounded-md bg-primary text-primary-foreground text-sm font-medium hover:bg-primary/90 disabled:opacity-60 transition-colors"
          >
            {adding ? <Loader2 className="w-3.5 h-3.5 animate-spin" /> : <Plus className="w-3.5 h-3.5" />}
            Add
          </button>
        </div>
        {addError && <p className="text-xs text-destructive">{addError}</p>}
      </form>
    </section>
  )
}
