<script lang="ts">
  import { router, matchPath } from './lib/router.svelte'
  import { ROUTES } from './lib/routes'
  import AppShell from './lib/AppShell.svelte'
  import Home from './routes/Home.svelte'
  import Library from './routes/Library.svelte'
  import Discover from './routes/Discover.svelte'
  import Queue from './routes/Queue.svelte'
  import Settings from './routes/Settings.svelte'
  import Login from './routes/Login.svelte'
  import Setup from './routes/Setup.svelte'
  import MangaDetail from './routes/MangaDetail.svelte'
  import Reader from './routes/Reader.svelte'

  const routeTable = [
    { pattern: ROUTES.home, component: Home, shell: true },
    { pattern: ROUTES.library, component: Library, shell: true },
    { pattern: '/title/:id', component: MangaDetail, shell: true },
    { pattern: ROUTES.discover, component: Discover, shell: true },
    { pattern: ROUTES.queue, component: Queue, shell: true },
    { pattern: ROUTES.settings, component: Settings, shell: true },
    { pattern: ROUTES.login, component: Login, shell: false },
    { pattern: ROUTES.setup, component: Setup, shell: false },
    { pattern: '/reader/:id', component: Reader, shell: false },
  ]

  function onUnauthorized() {
    router.navigate(ROUTES.login, { replace: true })
  }
  window.addEventListener('arrgh:unauthorized', onUnauthorized)

  const match = $derived.by(() => {
    for (const route of routeTable) {
      const params = matchPath(route.pattern, router.path)
      if (params) return { ...route, params }
    }
    return null
  })

  // Legacy /manga/:id -> /title/:id, unmatched paths -> home
  $effect(() => {
    if (match) return
    const legacy = matchPath('/manga/:id', router.path)
    router.navigate(legacy ? `/title/${legacy.id}` : ROUTES.home, { replace: true })
  })
</script>

{#if match}
  {#if match.shell}
    <AppShell>
      {#snippet children()}
        <match.component {...match.params} />
      {/snippet}
    </AppShell>
  {:else}
    <match.component {...match.params} />
  {/if}
{/if}
