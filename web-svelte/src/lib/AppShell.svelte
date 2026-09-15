<script lang="ts">
  import type { Snippet } from 'svelte'
  import { Home, BookOpen, Compass, Download, Settings, User, LogOut } from '@lucide/svelte'
  import { api, clearToken, getUsername, getRole, type VersionInfo } from './api'
  import { cn } from './utils'
  import { router } from './router.svelte'
  import { ROUTES } from './routes'

  let { children }: { children: Snippet } = $props()

  let version = $state<VersionInfo | null>(null)
  api.getVersion().then((v) => (version = v)).catch(() => {})

  const NAV_ITEMS = [
    { label: 'Home', icon: Home, path: ROUTES.home },
    { label: 'Library', icon: BookOpen, path: ROUTES.library },
    { label: 'Discover', icon: Compass, path: ROUTES.discover },
    { label: 'Downloads', icon: Download, path: ROUTES.queue },
    { label: 'Settings', icon: Settings, path: ROUTES.settings },
  ]

  function isActive(path: string) {
    return path === '/' ? router.path === '/' : router.path.startsWith(path)
  }

  function logout() {
    clearToken()
    router.navigate(ROUTES.login)
  }
</script>

<div class="flex h-full bg-background">
  <aside class="w-52 shrink-0 flex flex-col border-r border-border bg-card/80 backdrop-blur-xl">
    <div class="px-5 pt-5 pb-4">
      <h1 class="text-base font-bold leading-none">*ARRgh</h1>
      <p class="text-[11px] text-muted-foreground mt-1">Weeb Library</p>
    </div>

    <nav class="flex-1 px-3 space-y-0.5">
      {#each NAV_ITEMS as item (item.path)}
        <button
          data-nav
          onclick={() => router.navigate(item.path)}
          class={cn(
            'w-full flex items-center gap-3 px-3 py-2.5 rounded-lg text-sm font-medium transition-colors text-left',
            isActive(item.path)
              ? 'bg-accent text-foreground'
              : 'text-muted-foreground hover:bg-accent/60 hover:text-foreground',
          )}
        >
          <item.icon class="w-[15px] h-[15px] shrink-0" />
          <span class="flex-1">{item.label}</span>
        </button>
      {/each}
    </nav>

    <div>
      <div class="flex items-center gap-2.5 border-border border-t border-b p-4">
        <div class="w-8 h-8 rounded-full bg-muted flex items-center justify-center shrink-0">
          <User class="w-4 h-4 text-muted-foreground" />
        </div>
        <div class="min-w-0 flex-1">
          <p class="text-xs font-semibold truncate">{getUsername() ?? 'Self-Hosted'}</p>
          <p class="text-[10px] text-muted-foreground uppercase tracking-wide">
            {#if getRole() === 'admin'}
              <span class="text-primary">Admin</span>
            {:else}
              Member
            {/if}
          </p>
        </div>
        <button
          onclick={logout}
          class="w-7 h-7 flex items-center justify-center rounded-md text-muted-foreground hover:text-foreground hover:bg-accent/60 transition-colors shrink-0"
          title="Logout"
        >
          <LogOut class="w-3.5 h-3.5" />
        </button>
      </div>
      <div class="p-4">
        {#if version}
          {#if version.latest && version.release_url}
            <a
              href={version.release_url}
              target="_blank"
              rel="noopener noreferrer"
              class="block text-[10px] font-medium text-primary hover:text-primary/80 transition-colors text-center"
              title={`v${version.latest} available`}
            >
              v{version.current} ↑
            </a>
          {:else}
            <p class="text-[10px] text-muted-foreground/50 text-center">v{version.current}</p>
          {/if}
        {/if}
      </div>
    </div>
  </aside>

  <div class="flex-1 flex flex-col min-w-0 overflow-hidden">
    {@render children()}
  </div>
</div>
