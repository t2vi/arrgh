<script lang="ts">
  import { cn } from '../lib/utils'
  import { SetupStore } from '../lib/setup.svelte'
  import StepAccount from './setup/StepAccount.svelte'
  import StepSettings from './setup/StepSettings.svelte'

  let _params: Record<string, string> = $props()

  const store = new SetupStore()
</script>

<div class="min-h-screen bg-background flex items-center justify-center p-4">
  <div class="w-full max-w-sm space-y-8">
    <div class="text-center space-y-2">
      <h1 class="text-4xl font-black tracking-tight text-primary">*ARRgh</h1>
      {#if store.step === 1}
        <p class="text-sm font-semibold">Welcome — create your account</p>
        <p class="text-xs text-muted-foreground">One-time setup for your self-hosted library.</p>
      {:else}
        <p class="text-sm font-semibold">Configure your library</p>
        <p class="text-xs text-muted-foreground">These can be changed later in Settings.</p>
      {/if}
    </div>

    <div class="flex items-center justify-center gap-2">
      {#each [1, 2] as s (s)}
        <div class={cn('w-2 h-2 rounded-full transition-colors', store.step >= s ? 'bg-primary' : 'bg-muted')}></div>
      {/each}
    </div>

    {#if store.step === 1}
      <StepAccount onDone={() => store.goToStep2()} />
    {:else}
      <StepSettings onDone={() => store.finish()} />
    {/if}
  </div>
</div>
