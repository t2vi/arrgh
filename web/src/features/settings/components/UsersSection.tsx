import { useState, useEffect, useCallback } from 'react'
import { Loader2, ShieldCheck, Eye, Trash2 } from 'lucide-react'
import { api, type UserListItem } from '@/api'
import { Button } from '@/components/ui/button'
import { Input } from '@/components/ui/input'
import { cn } from '@/lib/utils'

function UserRow({ user, onPatch, onDelete }: {
  user: UserListItem
  onPatch: (patch: { role?: string; allow_explicit?: boolean }) => void
  onDelete: () => void
}) {
  const isSelf = localStorage.getItem('arrgh_username') === user.username

  return (
    <div className="flex items-center gap-2 px-3 py-2 rounded-lg bg-muted/50">
      <div className="flex-1 min-w-0">
        <p className="text-sm font-medium truncate">{user.username}</p>
      </div>

      <button
        type="button"
        title={user.role === 'admin' ? 'Admin — click to demote' : 'Member — click to promote'}
        onClick={() => !isSelf && onPatch({ role: user.role === 'admin' ? 'member' : 'admin' })}
        disabled={isSelf}
        className={cn(
          'flex items-center gap-1 px-2 py-0.5 rounded-full text-[10px] font-semibold transition-colors',
          user.role === 'admin'
            ? 'bg-primary/20 text-primary hover:bg-primary/30'
            : 'bg-muted text-muted-foreground hover:bg-accent',
          isSelf && 'opacity-50 cursor-default',
        )}
      >
        <ShieldCheck className="w-3 h-3" />
        {user.role}
      </button>

      <button
        type="button"
        title={user.allow_explicit ? 'Explicit allowed — click to revoke' : 'Explicit blocked — click to allow'}
        onClick={() => onPatch({ allow_explicit: !user.allow_explicit })}
        className={cn(
          'flex items-center gap-1 px-2 py-0.5 rounded-full text-[10px] font-semibold transition-colors',
          user.allow_explicit
            ? 'bg-orange-500/20 text-orange-400 hover:bg-orange-500/30'
            : 'bg-muted text-muted-foreground hover:bg-accent',
        )}
      >
        <Eye className="w-3 h-3" />
        18+
      </button>

      {!isSelf && (
        <button
          type="button"
          title="Delete user"
          onClick={onDelete}
          className="w-6 h-6 flex items-center justify-center rounded-md text-muted-foreground hover:text-destructive hover:bg-destructive/10 transition-colors"
        >
          <Trash2 className="w-3.5 h-3.5" />
        </button>
      )}
    </div>
  )
}

export function UsersSection() {
  const [users, setUsers] = useState<UserListItem[]>([])
  const [loading, setLoading] = useState(true)
  const [newUsername, setNewUsername] = useState('')
  const [newPassword, setNewPassword] = useState('')
  const [createError, setCreateError] = useState('')
  const [creating, setCreating] = useState(false)

  const reload = useCallback(() => {
    setLoading(true)
    api.listUsers()
      .then(setUsers)
      .catch(() => {})
      .finally(() => setLoading(false))
  }, [])

  useEffect(() => { reload() }, [reload])

  async function handlePatch(id: string, patch: Parameters<typeof api.patchUser>[1]) {
    await api.patchUser(id, patch)
    reload()
  }

  async function handleDelete(id: string) {
    await api.deleteUser(id)
    reload()
  }

  async function handleCreate(e: React.FormEvent) {
    e.preventDefault()
    setCreateError('')
    if (newPassword.length < 6) { setCreateError('Password must be at least 6 characters.'); return }
    setCreating(true)
    try {
      await api.createUser(newUsername.trim(), newPassword)
      setNewUsername('')
      setNewPassword('')
      reload()
    } catch (err: unknown) {
      const msg = err instanceof Error ? err.message : ''
      if (msg.includes('409')) setCreateError('Username already taken.')
      else setCreateError('Failed to create user.')
    } finally {
      setCreating(false)
    }
  }

  return (
    <section className="space-y-4">
      {loading ? (
        <Loader2 className="w-4 h-4 animate-spin text-muted-foreground" />
      ) : (
        <div className="space-y-1.5">
          {users.map((u) => (
            <UserRow
              key={u.id}
              user={u}
              onPatch={(patch) => handlePatch(u.id, patch)}
              onDelete={() => handleDelete(u.id)}
            />
          ))}
        </div>
      )}

      <form onSubmit={handleCreate} className="space-y-2 pt-2 border-t border-border">
        <p className="text-xs font-medium text-muted-foreground">Add member</p>
        <div className="flex gap-2">
          <Input
            placeholder="Username"
            value={newUsername}
            onChange={(e) => setNewUsername(e.target.value)}
            required
            className="flex-1"
          />
          <Input
            type="password"
            placeholder="Password"
            value={newPassword}
            onChange={(e) => setNewPassword(e.target.value)}
            required
            className="flex-1"
          />
          <Button type="submit" size="sm" disabled={creating}>
            {creating ? <Loader2 className="w-3.5 h-3.5 animate-spin" /> : 'Add'}
          </Button>
        </div>
        {createError && <p className="text-xs text-destructive">{createError}</p>}
      </form>
    </section>
  )
}
