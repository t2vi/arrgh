<script lang="ts">
  import { LoaderCircle, Check } from '@lucide/svelte'
  import { api } from '../../lib/api'
  import Button from '../../lib/components/ui/Button.svelte'
  import Input from '../../lib/components/ui/Input.svelte'

  let password = $state('')
  let confirm = $state('')
  let saving = $state(false)
  let error = $state('')
  let saved = $state(false)

  async function handleSubmit(e: SubmitEvent) {
    e.preventDefault()
    error = ''
    if (password.length < 6) { error = 'Min 6 characters.'; return }
    if (password !== confirm) { error = 'Passwords do not match.'; return }
    saving = true
    try {
      await api.changePassword(password)
      password = ''
      confirm = ''
      saved = true
      setTimeout(() => (saved = false), 2000)
    } catch {
      error = 'Failed to change password.'
    } finally {
      saving = false
    }
  }
</script>

<section class="space-y-3">
  <h2 class="text-xs font-semibold uppercase tracking-widest text-muted-foreground">Change Password</h2>
  <form onsubmit={handleSubmit} class="space-y-2 max-w-xs">
    <Input type="password" placeholder="New password" bind:value={password} autocomplete="new-password" required />
    <Input type="password" placeholder="Confirm password" bind:value={confirm} autocomplete="new-password" required />
    {#if error}
      <p class="text-xs text-destructive">{error}</p>
    {/if}
    <Button type="submit" size="sm" disabled={saving} class="gap-1.5">
      {#if saving}
        <LoaderCircle class="w-3.5 h-3.5 animate-spin" />
      {:else if saved}
        <Check class="w-3.5 h-3.5" />
      {/if}
      {saved ? 'Saved' : 'Update'}
    </Button>
  </form>
</section>
