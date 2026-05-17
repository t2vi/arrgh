import { useNavigate } from 'react-router-dom'
import { ChevronRight, Loader2 } from 'lucide-react'
import { Button } from '@/components/ui/button'
import { cn } from '@/lib/utils'
import { useSettings, type Tab } from './hooks/useSettings'
import { ServerSettingsSection } from './components/ServerSettingsSection'
import { UsersSection } from './components/UsersSection'
import { SourcesSection } from './components/SourcesSection'
import { ChangePasswordSection, ClientSection } from './components/AccountSection'

const ADMIN_TABS: { id: Tab; label: string }[] = [
  { id: 'library', label: 'Library' },
  { id: 'users',   label: 'Users'   },
  { id: 'sources', label: 'Sources' },
  { id: 'account', label: 'Account' },
]

export default function Settings() {
  const navigate = useNavigate()
  const h = useSettings()

  if (h.isLoading) {
    return (
      <div className="flex-1 flex items-center justify-center">
        <Loader2 className="w-5 h-5 animate-spin text-muted-foreground" />
      </div>
    )
  }

  return (
    <div className="flex flex-col h-full">
      <header className="flex items-center gap-3 px-4 py-3 border-b border-border bg-card shrink-0">
        <Button variant="ghost" size="icon" onClick={() => navigate(-1)}>
          <ChevronRight className="w-4 h-4 rotate-180" />
        </Button>
        <h1 className="text-base font-semibold">Settings</h1>

        {h.admin && (
          <div className="ml-4 flex rounded-lg bg-muted p-0.5 gap-0.5">
            {ADMIN_TABS.map((t) => (
              <button
                key={t.id}
                type="button"
                onClick={() => h.setTab(t.id)}
                className={cn(
                  'px-3 py-1 rounded-md text-xs font-semibold transition-colors',
                  h.tab === t.id
                    ? 'bg-card text-foreground shadow-sm'
                    : 'text-muted-foreground hover:text-foreground',
                )}
              >
                {t.label}
              </button>
            ))}
          </div>
        )}
      </header>

      <div className="flex-1 overflow-auto p-6 max-w-lg">
        {h.tab === 'library' && h.admin && (
          <div className="space-y-8">
            {h.settings
              ? <ServerSettingsSection settings={h.settings} saving={h.saving} onSave={h.handleSave} />
              : <p className="text-sm text-muted-foreground">Failed to load settings.</p>
            }
          </div>
        )}

        {h.tab === 'users' && h.admin && <UsersSection />}

        {h.tab === 'sources' && h.admin && <SourcesSection />}

        {h.tab === 'account' && (
          <div className="space-y-8">
            <ChangePasswordSection />
            <ClientSection />
            <section>
              <h2 className="text-xs font-semibold uppercase tracking-widest text-muted-foreground mb-3">Session</h2>
              <Button variant="outline" onClick={h.logout} className="text-destructive border-destructive/40 hover:bg-destructive/10">
                Sign out
              </Button>
            </section>
          </div>
        )}
      </div>
    </div>
  )
}
