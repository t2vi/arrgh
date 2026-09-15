<script lang="ts">
  import { api } from '../../lib/api'
  import SettingSegmentCard from './SettingSegmentCard.svelte'
  import SegmentControl from './SegmentControl.svelte'

  let { mangaId, value }: { mangaId: string; value: string | null } = $props()

  // svelte-ignore state_referenced_locally
  let current = $state(value)
  const options: { label: string; v: string | null }[] = [
    { label: 'Global', v: null },
    { label: 'Paged', v: 'paged' },
    { label: 'Scroll', v: 'scroll' },
  ]

  async function handleSelect(i: number) {
    const v = options[i].v
    current = v
    await api.setTitleReaderMode(mangaId, v).catch(() => {})
  }

  const description = $derived(
    current == null
      ? 'Uses the global reader mode setting.'
      : current === 'paged'
        ? 'One page at a time, tap/click to advance.'
        : 'All pages in a continuous vertical scroll.',
  )
</script>

<SettingSegmentCard label="Reader Mode" {description}>
  <SegmentControl
    options={options.map((o) => o.label)}
    active={options.findIndex((o) => o.v === current)}
    onSelect={handleSelect}
  />
</SettingSegmentCard>
