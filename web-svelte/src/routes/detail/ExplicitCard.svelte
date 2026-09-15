<script lang="ts">
  import { api } from '../../lib/api'

  let { mangaId, value }: { mangaId: string; value: boolean } = $props()

  // svelte-ignore state_referenced_locally
  let current = $state(value)

  async function toggle() {
    const next = !current
    current = next
    await api.setTitleExplicit(mangaId, next).catch(() => (current = !next))
  }
</script>

<div class="rounded-lg bg-card border border-border p-4 space-y-2">
  <div class="flex items-center justify-between gap-3">
    <p class="text-[10px] font-semibold uppercase tracking-wider text-muted-foreground">Explicit Content</p>
    <button
      type="button"
      onclick={toggle}
      aria-label="Toggle explicit content"
      class={`relative w-10 h-[22px] rounded-full transition-colors overflow-hidden ${current ? 'bg-orange-500' : 'bg-muted'}`}
    >
      <span
        class={`absolute top-[3px] left-0 w-4 h-4 rounded-full bg-white shadow transition-transform ${current ? 'translate-x-[21px]' : 'translate-x-[3px]'}`}
      ></span>
    </button>
  </div>
  <p class="text-xs text-muted-foreground">
    {current ? 'Marked 18+ — only users with explicit access can see this.' : 'Not marked explicit — visible to all users.'}
  </p>
</div>
