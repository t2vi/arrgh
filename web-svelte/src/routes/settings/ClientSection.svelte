<script lang="ts">
  import { Check } from '@lucide/svelte'
  import Button from '../../lib/components/ui/Button.svelte'
  import Input from '../../lib/components/ui/Input.svelte'

  // svelte-ignore state_referenced_locally
  let url = $state(localStorage.getItem('serverUrl') ?? '')
  let saved = $state(false)

  function save() {
    localStorage.setItem('serverUrl', url.trim().replace(/\/$/, ''))
    saved = true
    setTimeout(() => (saved = false), 2000)
  }
</script>

<section class="space-y-3">
  <h2 class="text-xs font-semibold uppercase tracking-widest text-muted-foreground">Server URL</h2>
  <p class="text-xs text-muted-foreground">Leave blank when the UI is served directly from *ARRgh.</p>
  <Input
    bind:value={url}
    placeholder="http://192.168.1.x:8282"
    onkeydown={(e: KeyboardEvent) => e.key === 'Enter' && save()}
    class="max-w-xs"
  />
  <Button onclick={save} size="sm" class="gap-1.5">
    {#if saved}
      <Check class="w-3.5 h-3.5" />
    {/if}
    {saved ? 'Saved' : 'Save'}
  </Button>
</section>
