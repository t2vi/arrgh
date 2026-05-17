import { useEffect, useState } from 'react'
import { Routes, Route, Navigate, useNavigate } from 'react-router-dom'
import AppLayout from './components/AppLayout'
import Home from './features/home'
import Library from './features/library'
import MangaDetail from './features/manga-detail'
import Reader from './features/reader'
import Settings from './features/settings'
import Discover from './features/discover'
import Queue from './features/queue'
import Login from './features/login'
import Setup from './features/setup'
import { api, getToken, clearToken } from './api'
import { ROUTES } from './lib/routes'

type AuthState = 'checking' | 'needs_setup' | 'needs_login' | 'authenticated'

function AuthGate({ children }: { children: React.ReactNode }) {
  const [state, setState] = useState<AuthState>('checking')
  const navigate = useNavigate()

  useEffect(() => {
    const token = getToken()
    if (!token) {
      api.authStatus()
        .then((s) => setState(s.needs_setup ? 'needs_setup' : 'needs_login'))
        .catch(() => setState('needs_setup'))
      return
    }

    api.me()
      .then(() => setState('authenticated'))
      .catch(() => {
        clearToken()
        setState('needs_login')
      })
  }, [])

  useEffect(() => {
    function onUnauthorized() {
      setState('needs_login')
    }
    window.addEventListener('arrgh:unauthorized', onUnauthorized)
    return () => window.removeEventListener('arrgh:unauthorized', onUnauthorized)
  }, [])

  useEffect(() => {
    if (state === 'needs_setup') navigate(ROUTES.setup, { replace: true })
    else if (state === 'needs_login') navigate(ROUTES.login, { replace: true })
  }, [state, navigate])

  if (state === 'checking') {
    return (
      <div className="min-h-screen bg-background flex items-center justify-center">
        <div className="w-6 h-6 rounded-full border-2 border-primary border-t-transparent animate-spin" />
      </div>
    )
  }

  if (state !== 'authenticated') return null

  return <>{children}</>
}

export default function App() {
  return (
    <Routes>
      {/* Public */}
      <Route path={ROUTES.login} element={<Login />} />
      <Route path={ROUTES.setup} element={<Setup />} />

      {/* Protected */}
      <Route element={
        <AuthGate>
          <AppLayout />
        </AuthGate>
      }>
        <Route path={ROUTES.home}     element={<Home />} />
        <Route path={ROUTES.library}  element={<Library />} />
        <Route path="/manga/:id"      element={<MangaDetail />} />
        <Route path={ROUTES.settings} element={<Settings />} />
        <Route path={ROUTES.discover} element={<Discover />} />
        <Route path={ROUTES.queue}    element={<Queue />} />
      </Route>

      {/* Reader — protected but no AppLayout */}
      <Route path="/reader/:chapterId" element={
        <AuthGate>
          <Reader />
        </AuthGate>
      } />

      <Route path="*" element={<Navigate to={ROUTES.home} replace />} />
    </Routes>
  )
}
