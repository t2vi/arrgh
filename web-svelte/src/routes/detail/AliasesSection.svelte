<script lang="ts">
  import { X } from '@lucide/svelte'

  let {
    aliases,
    pending,
    onAdd,
    onRemove,
  }: { aliases: string[]; pending: boolean; onAdd: (alias: string) => void; onRemove: (alias: string) => void } =
    $props()

  let draft = $state('')

  function submit() {
    if (!draft.trim()) return
    onAdd(draft.trim())
    draft = ''
  }
</script>

<div class="rounded-lg bg-card border border-border p-4 space-y-3">
  <p class="text-[10px] font-semibold uppercase tracking-wider text-muted-foreground">Also search as…</p>
  {#if aliases.length > 0}
    <ul class="flex flex-wrap gap-2">
      {#each aliases as alias (alias)}
        <li class="flex items-center gap-1.5 bg-muted rounded-full pl-3 pr-1.5 py-1 text-xs">
          {alias}
          <button
            onclick={() => onRemove(alias)}
            disabled={pending}
            aria-label={`Remove alias ${alias}`}
            class="text-muted-foreground hover:text-foreground transition-colors"
          >
            <X class="w-3 h-3" />
          </button>
        </li>
      {/each}
    </ul>
  {/if}
  <div class="flex gap-2">
    <input
      type="text"
      bind:value={draft}
      onkeydown={(e) => {
        if (e.key === 'Enter') submit()
      }}
      placeholder="Alternate title a source might use…"
      class="flex-1 bg-muted border border-transparent focus:border-ring rounded-md px-3 py-1.5 text-xs outline-none transition-colors"
    />
    <button
      onclick={submit}
      disabled={pending || !draft.trim()}
      class="px-3 py-1.5 rounded-md bg-primary text-primary-foreground text-xs font-semibold hover:opacity-90 transition-opacity disabled:opacity-50"
    >
      Add
    </button>
  </div>
  <p class="text-[10px] text-muted-foreground leading-relaxed">
    Adding an alias re-checks every source using it as a search term — useful when a source lists this title under a
    different name.
  </p>
</div>
