<script lang="ts">
  import { LoaderCircle, ShieldCheck, Eye, Trash2 } from '@lucide/svelte'
  import { api, getUsername, type UserListItem } from '../../lib/api'
  import Button from '../../lib/components/ui/Button.svelte'
  import Input from '../../lib/components/ui/Input.svelte'
  import { cn } from '../../lib/utils'

  let users = $state<UserListItem[]>([])
  let loading = $state(true)
  let newUsername = $state('')
  let newPassword = $state('')
  let createError = $state('')
  let creating = $state(false)

  function reload() {
    loading = true
    api.listUsers()
      .then((d) => (users = d))
      .catch(() => {})
      .finally(() => (loading = false))
  }

  reload()

  async function handlePatch(id: string, patch: { role?: string; allow_explicit?: boolean }) {
    await api.patchUser(id, patch)
    reload()
  }

  async function handleDelete(id: string) {
    await api.deleteUser(id)
    reload()
  }

  async function handleCreate(e: SubmitEvent) {
    e.preventDefault()
    createError = ''
    if (newPassword.length < 6) { createError = 'Password must be at least 6 characters.'; return }
    creating = true
    try {
      await api.createUser(newUsername.trim(), newPassword)
      newUsername = ''
      newPassword = ''
      reload()
    } catch (err: unknown) {
      const msg = err instanceof Error ? err.message : ''
      createError = msg.includes('409') ? 'Username already taken.' : 'Failed to create user.'
    } finally {
      creating = false
    }
  }
</script>

<section class="space-y-4">
  {#if loading}
    <LoaderCircle class="w-4 h-4 animate-spin text-muted-foreground" />
  {:else}
    <div class="space-y-1.5">
      {#each users as user (user.id)}
        {@const isSelf = getUsername() === user.username}
        <div class="flex items-center gap-2 px-3 py-2 rounded-lg bg-muted/50">
          <div class="flex-1 min-w-0">
            <p class="text-sm font-medium truncate">{user.username}</p>
          </div>

          <button
            type="button"
            title={user.role === 'admin' ? 'Admin — click to demote' : 'Member — click to promote'}
            onclick={() => !isSelf && handlePatch(user.id, { role: user.role === 'admin' ? 'member' : 'admin' })}
            disabled={isSelf}
            class={cn(
              'flex items-center gap-1 px-2 py-0.5 rounded-full text-[10px] font-semibold transition-colors',
              user.role === 'admin'
                ? 'bg-primary/20 text-primary hover:bg-primary/30'
                : 'bg-muted text-muted-foreground hover:bg-accent',
              isSelf && 'opacity-50 cursor-default',
            )}
          >
            <ShieldCheck class="w-3 h-3" />
            {user.role}
          </button>

          <button
            type="button"
            title={user.allow_explicit ? 'Explicit allowed — click to revoke' : 'Explicit blocked — click to allow'}
            onclick={() => handlePatch(user.id, { allow_explicit: !user.allow_explicit })}
            class={cn(
              'flex items-center gap-1 px-2 py-0.5 rounded-full text-[10px] font-semibold transition-colors',
              user.allow_explicit
                ? 'bg-orange-500/20 text-orange-400 hover:bg-orange-500/30'
                : 'bg-muted text-muted-foreground hover:bg-accent',
            )}
          >
            <Eye class="w-3 h-3" />
            18+
          </button>

          {#if !isSelf}
            <button
              type="button"
              title="Delete user"
              onclick={() => handleDelete(user.id)}
              class="w-6 h-6 flex items-center justify-center rounded-md text-muted-foreground hover:text-destructive hover:bg-destructive/10 transition-colors"
            >
              <Trash2 class="w-3.5 h-3.5" />
            </button>
          {/if}
        </div>
      {/each}
    </div>
  {/if}

  <form onsubmit={handleCreate} class="space-y-2 pt-2 border-t border-border">
    <p class="text-xs font-medium text-muted-foreground">Add member</p>
    <div class="flex gap-2">
      <Input placeholder="Username" bind:value={newUsername} required class="flex-1" />
      <Input type="password" placeholder="Password" bind:value={newPassword} required class="flex-1" />
      <Button type="submit" size="sm" disabled={creating}>
        {#if creating}
          <LoaderCircle class="w-3.5 h-3.5 animate-spin" />
        {:else}
          Add
        {/if}
      </Button>
    </div>
    {#if createError}
      <p class="text-xs text-destructive">{createError}</p>
    {/if}
  </form>
</section>
