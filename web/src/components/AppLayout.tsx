import { useState, useEffect, useCallback } from 'react'
import { useLocation, useNavigate, Outlet } from 'react-router-dom'
import { Home, BookOpen, Compass, Download, Settings, User, LogOut } from 'lucide-react'
import { api, clearToken, getUsername, getRole, type QueueItem } from '@/api'
import { cn } from '@/lib/utils'
import { useDpadNav } from '@/hooks/useDpadNav'
import { ROUTES } from '@/lib/routes'

const NAV_ITEMS = [
  { label: 'Home',      icon: Home,     path: ROUTES.home     },
  { label: 'Library',   icon: BookOpen, path: ROUTES.library  },
  { label: 'Discover',  icon: Compass,  path: ROUTES.discover },
  { label: 'Downloads', icon: Download, path: ROUTES.queue    },
  { label: 'Settings',  icon: Settings, path: ROUTES.settings },
]

export default function AppLayout() {
  const location = useLocation()
  const navigate = useNavigate()
  useDpadNav()

  const [queueData, setQueueData] = useState<QueueItem[]>([])

  const fetchQueue = useCallback(() => {
    api.getQueue().then(setQueueData).catch(() => {})
  }, [])

  useEffect(() => {
    fetchQueue()
    const id = setInterval(fetchQueue, 3000)
    return () => clearInterval(id)
  }, [fetchQueue])

  const activeDownloads = queueData.filter(
    (i) => i.status === 'pending' || i.status === 'downloading'
  ).length

  return (
    <div className="flex h-full bg-background">
      <aside className="w-52 shrink-0 flex flex-col border-r border-border bg-card/80 backdrop-blur-xl">
        <div className="px-5 pt-5 pb-4">
          <h1 className="text-base font-bold leading-none">*ARRgh</h1>
          <p className="text-[11px] text-muted-foreground mt-1">Weeb Library</p>
        </div>

        <nav className="flex-1 px-3 space-y-0.5">
          {NAV_ITEMS.map(({ label, icon: Icon, path }) => {
            const active = path === '/'
              ? location.pathname === '/'
              : location.pathname.startsWith(path)
            return (
              <button
                key={path}
                data-nav
                onClick={() => navigate(path)}
                className={cn(
                  'w-full flex items-center gap-3 px-3 py-2.5 rounded-lg text-sm font-medium transition-colors text-left',
                  active
                    ? 'bg-accent text-foreground'
                    : 'text-muted-foreground hover:bg-accent/60 hover:text-foreground',
                )}
              >
                <Icon className="w-[15px] h-[15px] shrink-0" />
                <span className="flex-1">{label}</span>
                {path === ROUTES.queue && activeDownloads > 0 && (
                  <span className="bg-primary text-primary-foreground text-[9px] font-bold rounded-full min-w-[16px] h-[16px] flex items-center justify-center px-1 leading-none">
                    {activeDownloads}
                  </span>
                )}
              </button>
            )
          })}
        </nav>

        <div>
          <div className="flex items-center gap-2.5 border-border border-t border-b p-4">
            <div className="w-8 h-8 rounded-full bg-muted flex items-center justify-center shrink-0">
              <User className="w-4 h-4 text-muted-foreground" />
            </div>
            <div className="min-w-0 flex-1">
              <p className="text-xs font-semibold truncate">{getUsername() ?? 'Self-Hosted'}</p>
              <p className="text-[10px] text-muted-foreground uppercase tracking-wide">
                {getRole() === 'admin' ? <span className="text-primary">Admin</span> : 'Member'}
              </p>
            </div>
            <button
              onClick={() => { clearToken(); navigate(ROUTES.login) }}
              className="w-7 h-7 flex items-center justify-center rounded-md text-muted-foreground hover:text-foreground hover:bg-accent/60 transition-colors shrink-0"
              title="Logout"
            >
              <LogOut className="w-3.5 h-3.5" />
            </button>
          </div>
          <div className="p-4">
          <a
            href="https://www.gnu.org/licenses/gpl-3.0.html"
            target="_blank"
            rel="noopener noreferrer"
            className="block text-[10px] text-muted-foreground/50 hover:text-muted-foreground transition-colors text-center"
          >
            GNU GPL v3
          </a>
          </div>
        </div>
      </aside>

      <div className="flex-1 flex flex-col min-w-0 overflow-hidden">
        <Outlet />
      </div>
    </div>
  )
}
