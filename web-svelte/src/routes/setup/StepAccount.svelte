<script lang="ts">
  import { LoaderCircle } from '@lucide/svelte'
  import { api, setToken } from '../../lib/api'
  import Button from '../../lib/components/ui/Button.svelte'
  import Input from '../../lib/components/ui/Input.svelte'

  let { onDone }: { onDone: () => void } = $props()

  let username = $state('')
  let password = $state('')
  let confirm = $state('')
  let error = $state('')
  let loading = $state(false)

  async function handleSubmit(e: SubmitEvent) {
    e.preventDefault()
    error = ''
    if (password.length < 6) {
      error = 'Password must be at least 6 characters.'
      return
    }
    if (password !== confirm) {
      error = 'Passwords do not match.'
      return
    }
    loading = true
    try {
      const res = await api.register(username.trim(), password)
      setToken(res.token, res.username, res.role, res.allow_explicit)
      onDone()
    } catch (err: unknown) {
      const msg = err instanceof Error ? err.message : ''
      if (msg.includes('409') || msg.includes('Conflict')) error = 'Username already taken.'
      else if (msg.includes('403')) error = 'Setup already complete. Please sign in.'
      else error = 'Setup failed. Please try again.'
    } finally {
      loading = false
    }
  }
</script>

<form onsubmit={handleSubmit} class="space-y-4">
  <div class="space-y-2">
    <label for="setup-username" class="text-sm font-medium">Username</label>
    <Input id="setup-username" type="text" autocomplete="username" bind:value={username} placeholder="Username" required autofocus />
  </div>
  <div class="space-y-2">
    <label for="setup-password" class="text-sm font-medium">Password</label>
    <Input id="setup-password" type="password" autocomplete="new-password" bind:value={password} placeholder="Min. 6 characters" required />
  </div>
  <div class="space-y-2">
    <label for="setup-confirm" class="text-sm font-medium">Confirm Password</label>
    <Input id="setup-confirm" type="password" autocomplete="new-password" bind:value={confirm} placeholder="Repeat password" required />
  </div>
  {#if error}
    <p class="text-sm text-destructive">{error}</p>
  {/if}
  <Button type="submit" class="w-full" disabled={loading}>
    {#if loading}<LoaderCircle class="w-4 h-4 animate-spin mr-2" />{/if}
    Create Account
  </Button>
</form>
