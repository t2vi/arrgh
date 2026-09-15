<script lang="ts">
  import { api, getUsername } from '../../lib/api'
  import type { Title } from '../../lib/types'

  function greeting(): string {
    const hour = new Date().getHours()
    if (hour < 12) return 'Good morning'
    if (hour < 17) return 'Good afternoon'
    return 'Good evening'
  }

  const TYPE_LABELS: Record<string, string> = { manga: 'manga', manhwa: 'manhwa', manhua: 'manhua', novel: 'novel' }

  function libraryLabel(typeCounts: Record<string, number>): string {
    const entries = Object.entries(typeCounts).filter(([, n]) => n > 0)
    if (entries.length === 0) return ''
    return entries.map(([type, n]) => `${n} ${TYPE_LABELS[type] ?? type}`).join(' · ') + ' in library'
  }

  let {
    typeCounts,
    totalRead,
    coverManga,
  }: {
    typeCounts: Record<string, number>
    totalRead: number
    coverManga: Pick<Title, 'id' | 'cover_url'> | null
  } = $props()

  const totalManga = $derived(Object.values(typeCounts).reduce((a, b) => a + b, 0))
  let failed = $state(false)
  const username = getUsername()
  const coverSrc = $derived(
    !failed && coverManga
      ? coverManga.cover_url?.startsWith('http')
        ? coverManga.cover_url
        : api.coverUrl(coverManga.id)
      : null,
  )
</script>

<div class="relative overflow-hidden" style="min-height: 172px">
  {#if coverSrc}
    <img
      src={coverSrc}
      alt=""
      class="absolute inset-0 w-full h-full object-cover object-top scale-110"
      style="filter: blur(32px); opacity: 0.18"
      onerror={() => (failed = true)}
    />
  {/if}
  <div class="absolute inset-0 bg-gradient-to-b from-background/60 via-background/80 to-background"></div>
  <div class="absolute inset-0 bg-gradient-to-r from-primary/5 to-transparent"></div>
  <div class="relative z-10 px-8 pt-10 pb-8">
    <p class="text-[11px] font-bold uppercase tracking-[0.25em] text-primary mb-2">*ARRgh</p>
    <h1 class="text-4xl font-extrabold tracking-tight leading-none mb-3">
      {greeting()}{username ? `, ${username}` : ''}.
    </h1>
    <div class="flex items-center gap-3 text-sm text-muted-foreground flex-wrap">
      {#if totalManga > 0}
        <span class="flex items-center gap-1.5">
          <span class="w-1 h-1 rounded-full bg-primary inline-block"></span>
          <span class="font-semibold text-foreground">{libraryLabel(typeCounts)}</span>
        </span>
      {/if}
      {#if totalRead > 0}
        <span class="flex items-center gap-1.5">
          <span class="w-1 h-1 rounded-full bg-muted-foreground inline-block"></span>
          <span><span class="font-semibold text-foreground">{totalRead}</span> chapters read</span>
        </span>
      {/if}
      {#if totalManga === 0}
        <span class="text-muted-foreground">Your library is empty — discover some manga to get started.</span>
      {/if}
    </div>
  </div>
</div>
