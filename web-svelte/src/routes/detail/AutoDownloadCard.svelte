<script lang="ts">
  import { api } from '../../lib/api'
  import SettingSegmentCard from './SettingSegmentCard.svelte'
  import SegmentControl from './SegmentControl.svelte'

  let { mangaId, value }: { mangaId: string; value: boolean | null } = $props()

  // svelte-ignore state_referenced_locally
  let current = $state(value)
  const options: { label: string; v: boolean | null }[] = [
    { label: 'Global', v: null },
    { label: 'Always', v: true },
    { label: 'Never', v: false },
  ]

  async function handleSelect(i: number) {
    const v = options[i].v
    current = v
    await api.setTitleAutoDownload(mangaId, v).catch(() => {})
  }

  const description = $derived(
    current === null
      ? 'Follows the global auto-download setting.'
      : current
        ? 'New chapters download automatically.'
        : 'New chapters are never auto-downloaded.',
  )
</script>

<SettingSegmentCard label="Auto-download" {description}>
  <SegmentControl
    options={options.map((o) => o.label)}
    active={options.findIndex((o) => o.v === current)}
    onSelect={handleSelect}
  />
</SettingSegmentCard>
