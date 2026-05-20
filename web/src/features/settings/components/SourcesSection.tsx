import { useState, useEffect, useCallback } from 'react'
import { Loader2, Globe, KeyRound, Plus, Trash2, PackageSearch, Download, CheckCircle2, X } from 'lucide-react'
import { api, type SourceRow, type PluginIndexEntry } from '@/api'
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
            <span title="Uses API key"><KeyRound className="w-3 h-3 text-muted-foreground shrink-0" /></span>
          )}
          {source.is_community && (
            <span className="px-1.5 py-0 rounded-full bg-primary/15 text-primary text-[10px] font-medium shrink-0">community</span>
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

      {source.is_community && (
        <button
          type="button"
          title="Remove plugin"
          onClick={() => onDelete(source.id)}
          className="w-6 h-6 flex items-center justify-center rounded-md text-muted-foreground hover:text-destructive hover:bg-destructive/10 transition-colors"
        >
          <Trash2 className="w-3.5 h-3.5" />
        </button>
      )}
    </div>
  )
}

function BrowseModal({ installedIds, onInstall, onClose }: {
  installedIds: Set<string>
  onInstall: (id: string) => Promise<void>
  onClose: () => void
}) {
  const [entries, setEntries] = useState<PluginIndexEntry[]>([])
  const [loading, setLoading] = useState(true)
  const [installing, setInstalling] = useState<string | null>(null)
  const [error, setError] = useState('')

  useEffect(() => {
    api.listPluginIndex()
      .then(setEntries)
      .catch(() => setError('Failed to load plugin index.'))
      .finally(() => setLoading(false))
  }, [])

  async function handleInstall(id: string) {
    setInstalling(id)
    try {
      await onInstall(id)
    } finally {
      setInstalling(null)
    }
  }

  return (
    <div
      className="fixed inset-0 z-50 flex items-center justify-center bg-black/60 backdrop-blur-sm"
      onClick={(e) => { if (e.target === e.currentTarget) onClose() }}
    >
      <div className="relative w-full max-w-lg mx-4 bg-card border border-border rounded-2xl shadow-2xl overflow-hidden">
        <div className="flex items-center justify-between px-5 py-4 border-b border-border">
          <div className="flex items-center gap-2">
            <PackageSearch className="w-4 h-4 text-primary" />
            <h2 className="text-sm font-semibold">Browse Plugins</h2>
          </div>
          <button
            type="button"
            onClick={onClose}
            className="w-6 h-6 flex items-center justify-center rounded-md text-muted-foreground hover:text-foreground hover:bg-muted transition-colors"
          >
            <X className="w-4 h-4" />
          </button>
        </div>

        <div className="max-h-[60vh] overflow-y-auto p-4 space-y-2">
          {loading ? (
            <div className="flex justify-center py-8">
              <Loader2 className="w-5 h-5 animate-spin text-muted-foreground" />
            </div>
          ) : error ? (
            <p className="text-sm text-destructive text-center py-4">{error}</p>
          ) : entries.length === 0 ? (
            <p className="text-sm text-muted-foreground text-center py-4">No plugins found.</p>
          ) : entries.map((entry) => {
            const isInstalled = installedIds.has(entry.id)
            const isBusy = installing === entry.id
            const canInstall = !entry.bundled && !!entry.download_url && !isInstalled

            return (
              <div key={entry.id} className="flex items-start gap-3 px-3 py-2.5 rounded-lg bg-muted/40 hover:bg-muted/70 transition-colors">
                <div className="flex-1 min-w-0">
                  <div className="flex items-center gap-2 flex-wrap">
                    <p className="text-sm font-medium">{entry.name}</p>
                    <span className="text-[10px] text-muted-foreground">v{entry.version}</span>
                    {entry.bundled && (
                      <span className="px-1.5 rounded-full bg-muted border border-border text-[10px] text-muted-foreground">bundled</span>
                    )}
                    {entry.default_explicit && (
                      <span className="px-1.5 rounded-full bg-destructive/15 text-destructive text-[10px]">18+</span>
                    )}
                    {entry.content_types.map((ct) => (
                      <span key={ct} className="px-1.5 rounded-full bg-card border border-border text-[10px] text-muted-foreground">
                        {ct}
                      </span>
                    ))}
                  </div>
                  {entry.description && (
                    <p className="text-xs text-muted-foreground mt-0.5">{entry.description}</p>
                  )}
                </div>

                {isInstalled ? (
                  <div className="flex items-center gap-1 text-xs text-primary shrink-0 mt-0.5">
                    <CheckCircle2 className="w-3.5 h-3.5" />
                    <span>Installed</span>
                  </div>
                ) : canInstall ? (
                  <button
                    type="button"
                    disabled={isBusy}
                    onClick={() => handleInstall(entry.id)}
                    className="flex items-center gap-1 px-2.5 py-1 rounded-md bg-primary text-primary-foreground text-xs font-medium hover:bg-primary/90 disabled:opacity-60 transition-colors shrink-0"
                  >
                    {isBusy ? <Loader2 className="w-3 h-3 animate-spin" /> : <Download className="w-3 h-3" />}
                    Install
                  </button>
                ) : null}
              </div>
            )
          })}
        </div>
      </div>
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
  const [browseOpen, setBrowseOpen] = useState(false)

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

  async function handleInstall(plugin_id: string) {
    await api.installPlugin(plugin_id)
    reload()
  }

  const installedPluginIds = new Set(
    sources.map((s) => {
      const parts = s.base_url.split('/')
      return parts[parts.length - 1]
    })
  )

  return (
    <section className="space-y-4">
      <p className="text-xs text-muted-foreground">
        External source plugins extend *ARRgh with additional manga and novel sources.
      </p>

      <div className="flex items-center justify-between">
        <span className="text-xs font-medium text-muted-foreground">Installed sources</span>
        <button
          type="button"
          onClick={() => setBrowseOpen(true)}
          className="flex items-center gap-1.5 px-2.5 py-1 rounded-md bg-muted border border-border text-xs font-medium text-foreground hover:bg-muted/70 transition-colors"
        >
          <PackageSearch className="w-3.5 h-3.5 text-primary" />
          Browse plugins
        </button>
      </div>

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
        <p className="text-xs font-medium text-muted-foreground">Add custom source</p>
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

      {browseOpen && (
        <BrowseModal
          installedIds={installedPluginIds}
          onInstall={handleInstall}
          onClose={() => setBrowseOpen(false)}
        />
      )}
    </section>
  )
}
