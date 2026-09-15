<script lang="ts">
  import { ChevronRight, LoaderCircle } from '@lucide/svelte'
  import { SettingsStore, type Tab } from '../lib/settings.svelte'
  import Button from '../lib/components/ui/Button.svelte'
  import { cn } from '../lib/utils'
  import ServerSettingsSection from './settings/ServerSettingsSection.svelte'
  import UsersSection from './settings/UsersSection.svelte'
  import SourcesSection from './settings/SourcesSection.svelte'
  import LogsSection from './settings/LogsSection.svelte'
  import ChangePasswordSection from './settings/ChangePasswordSection.svelte'
  import ClientSection from './settings/ClientSection.svelte'

  let _params: Record<string, string> = $props()

  const store = new SettingsStore()

  const ADMIN_TABS: { id: Tab; label: string }[] = [
    { id: 'library', label: 'Library' },
    { id: 'users', label: 'Users' },
    { id: 'sources', label: 'Sources' },
    { id: 'logs', label: 'Logs' },
    { id: 'account', label: 'Account' },
  ]
</script>

{#if store.isLoading}
  <div class="flex-1 flex items-center justify-center">
    <LoaderCircle class="w-5 h-5 animate-spin text-muted-foreground" />
  </div>
{:else}
  <div class="flex flex-col h-full">
    <header class="flex items-center gap-3 px-4 py-3 border-b border-border bg-card shrink-0">
      <Button variant="ghost" size="icon" onclick={() => window.history.back()}>
        <ChevronRight class="w-4 h-4 rotate-180" />
      </Button>
      <h1 class="text-base font-semibold">Settings</h1>

      {#if store.admin}
        <div class="ml-4 flex rounded-lg bg-muted p-0.5 gap-0.5">
          {#each ADMIN_TABS as t (t.id)}
            <button
              type="button"
              onclick={() => store.setTab(t.id)}
              class={cn(
                'px-3 py-1 rounded-md text-xs font-semibold transition-colors',
                store.tab === t.id ? 'bg-card text-foreground shadow-sm' : 'text-muted-foreground hover:text-foreground',
              )}
            >
              {t.label}
            </button>
          {/each}
        </div>
      {/if}
    </header>

    <div class="flex-1 overflow-auto p-6 max-w-lg">
      {#if store.tab === 'library' && store.admin}
        <div class="space-y-8">
          {#if store.settings}
            <ServerSettingsSection settings={store.settings} saving={store.saving} onSave={store.handleSave} />
          {:else}
            <p class="text-sm text-muted-foreground">Failed to load settings.</p>
          {/if}
        </div>
      {/if}

      {#if store.tab === 'users' && store.admin}
        <UsersSection />
      {/if}

      {#if store.tab === 'sources' && store.admin}
        <SourcesSection />
      {/if}

      {#if store.tab === 'logs' && store.admin}
        <LogsSection />
      {/if}

      {#if store.tab === 'account'}
        <div class="space-y-8">
          <ChangePasswordSection />
          <ClientSection />
          <section>
            <h2 class="text-xs font-semibold uppercase tracking-widest text-muted-foreground mb-3">Session</h2>
            <Button variant="outline" onclick={store.logout} class="text-destructive border-destructive/40 hover:bg-destructive/10">
              Sign out
            </Button>
          </section>
        </div>
      {/if}
    </div>
  </div>
{/if}
