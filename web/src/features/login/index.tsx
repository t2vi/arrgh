import { Loader2 } from 'lucide-react'
import { Button } from '@/components/ui/button'
import { Input } from '@/components/ui/input'
import { useLogin } from './hooks/useLogin'

export default function Login() {
  const h = useLogin()

  return (
    <div className="min-h-screen bg-background flex items-center justify-center p-4">
      <div className="w-full max-w-sm space-y-8">
        <div className="text-center space-y-1">
          <h1 className="text-4xl font-black tracking-tight text-primary">*ARRgh</h1>
          <p className="text-sm text-muted-foreground">Sign in to your library</p>
        </div>

        <form onSubmit={h.handleSubmit} className="space-y-4">
          <div className="space-y-2">
            <label className="text-sm font-medium text-foreground">Username</label>
            <Input
              type="text"
              autoComplete="username"
              value={h.username}
              onChange={(e) => h.setUsername(e.target.value)}
              placeholder="username"
              required
              autoFocus
            />
          </div>
          <div className="space-y-2">
            <label className="text-sm font-medium text-foreground">Password</label>
            <Input
              type="password"
              autoComplete="current-password"
              value={h.password}
              onChange={(e) => h.setPassword(e.target.value)}
              placeholder="••••••••"
              required
            />
          </div>

          {h.error && <p className="text-sm text-destructive">{h.error}</p>}

          <Button type="submit" className="w-full" disabled={h.loading}>
            {h.loading ? <Loader2 className="w-4 h-4 animate-spin mr-2" /> : null}
            Sign In
          </Button>
        </form>
      </div>
    </div>
  )
}
