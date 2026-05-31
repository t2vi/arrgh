import { RefreshCw, Loader2 } from 'lucide-react'

interface Props {
  hasSyncWarnings: boolean
  isSyncing: boolean
  isPending: boolean
  isRemoteSource: boolean
  onSync: () => void
}

export function NoChaptersMessage({ hasSyncWarnings, isSyncing, isPending, isRemoteSource, onSync }: Props) {
  if (isSyncing || isPending) {
    return (
      <p className="text-sm text-muted-foreground py-4 flex items-center gap-2">
        <RefreshCw className="w-3.5 h-3.5 animate-spin" /> Fetching chapters…
      </p>
    )
  }

  if (hasSyncWarnings) {
    return (
      <p className="text-sm text-muted-foreground py-4">
        No sources found for this title — none of the available plugins carry it.{' '}
        {isRemoteSource && (
          <button onClick={onSync} className="underline hover:text-foreground">
            Retry.
          </button>
        )}
      </p>
    )
  }

  return (
    <p className="text-sm text-muted-foreground py-4">
      No chapters.{isRemoteSource && (
        <button onClick={onSync} className="underline hover:text-foreground ml-1">
          Sync.
        </button>
      )}
    </p>
  )
}
