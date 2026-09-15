<script lang="ts">
  import { api } from '../../lib/api'

  let { coverUrl, mangaId }: { coverUrl: string | null; mangaId: string } = $props()

  let failed = $state(false)
  const src = $derived(!failed && coverUrl?.startsWith('http') ? coverUrl : api.coverUrl(mangaId))
</script>

{#if failed}
  <div class="w-36 shrink-0 rounded-xl aspect-[2/3] bg-muted/60 flex items-center justify-center text-4xl shadow-2xl ring-1 ring-white/10">
    📖
  </div>
{:else}
  <img
    src={src}
    alt=""
    class="w-36 shrink-0 rounded-xl aspect-[2/3] object-cover bg-muted shadow-2xl ring-1 ring-white/10"
    onerror={() => (failed = true)}
  />
{/if}
