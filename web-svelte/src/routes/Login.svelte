<script lang="ts">
  import { LoaderCircle } from '@lucide/svelte'
  import Button from '../lib/components/ui/Button.svelte'
  import Input from '../lib/components/ui/Input.svelte'
  import { LoginStore } from '../lib/login.svelte'
  import { api } from '../lib/api'
  import { router } from '../lib/router.svelte'
  import { ROUTES } from '../lib/routes'

  let _params: Record<string, string> = $props()

  const store = new LoginStore()

  // First-run guard — a fresh install with no admin yet sends /login to /setup instead.
  api.authStatus()
    .then((s) => {
      if (s.needs_setup) router.navigate(ROUTES.setup, { replace: true })
    })
    .catch(() => {})
</script>

<div class="min-h-screen bg-background flex items-center justify-center p-4">
  <div class="w-full max-w-sm space-y-8">
    <div class="text-center space-y-1">
      <h1 class="text-4xl font-black tracking-tight text-primary">*ARRgh</h1>
      <p class="text-sm text-muted-foreground">Sign in to your library</p>
    </div>

    <form onsubmit={(e) => store.submit(e)} class="space-y-4">
      <div class="space-y-2">
        <label for="login-username" class="text-sm font-medium text-foreground">Username</label>
        <Input
          id="login-username"
          type="text"
          autocomplete="username"
          bind:value={store.username}
          placeholder="Username"
          required
          autofocus
        />
      </div>
      <div class="space-y-2">
        <label for="login-password" class="text-sm font-medium text-foreground">Password</label>
        <Input
          id="login-password"
          type="password"
          autocomplete="current-password"
          bind:value={store.password}
          placeholder="••••••••"
          required
        />
      </div>

      {#if store.error}
        <p class="text-sm text-destructive">{store.error}</p>
      {/if}

      <Button type="submit" class="w-full" disabled={store.loading}>
        {#if store.loading}<LoaderCircle class="w-4 h-4 animate-spin mr-2" />{/if}
        Sign In
      </Button>
    </form>
  </div>
</div>
