import { useState } from 'react'
import { Loader2 } from 'lucide-react'
import { api, setToken } from '@/api'
import { Button } from '@/components/ui/button'
import { Input } from '@/components/ui/input'

export function StepAccount({ onDone }: { onDone: () => void }) {
  const [username, setUsername] = useState('')
  const [password, setPassword] = useState('')
  const [confirm, setConfirm] = useState('')
  const [error, setError] = useState('')
  const [loading, setLoading] = useState(false)

  async function handleSubmit(e: React.FormEvent) {
    e.preventDefault()
    setError('')
    if (password.length < 6) { setError('Password must be at least 6 characters.'); return }
    if (password !== confirm) { setError('Passwords do not match.'); return }
    setLoading(true)
    try {
      const res = await api.register(username.trim(), password)
      setToken(res.token, res.username, res.role, res.allow_explicit)
      onDone()
    } catch (err: unknown) {
      const msg = err instanceof Error ? err.message : ''
      if (msg.includes('409') || msg.includes('Conflict')) setError('Username already taken.')
      else if (msg.includes('403')) setError('Setup already complete. Please sign in.')
      else setError('Setup failed. Please try again.')
    } finally {
      setLoading(false)
    }
  }

  return (
    <form onSubmit={handleSubmit} className="space-y-4">
      <div className="space-y-2">
        <label className="text-sm font-medium">Username</label>
        <Input type="text" autoComplete="username" value={username}
          onChange={(e) => setUsername(e.target.value)} placeholder="username" required autoFocus />
      </div>
      <div className="space-y-2">
        <label className="text-sm font-medium">Password</label>
        <Input type="password" autoComplete="new-password" value={password}
          onChange={(e) => setPassword(e.target.value)} placeholder="min 6 characters" required />
      </div>
      <div className="space-y-2">
        <label className="text-sm font-medium">Confirm Password</label>
        <Input type="password" autoComplete="new-password" value={confirm}
          onChange={(e) => setConfirm(e.target.value)} placeholder="repeat password" required />
      </div>
      {error && <p className="text-sm text-destructive">{error}</p>}
      <Button type="submit" className="w-full" disabled={loading}>
        {loading && <Loader2 className="w-4 h-4 animate-spin mr-2" />}
        Create Account
      </Button>
    </form>
  )
}
