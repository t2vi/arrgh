<script lang="ts">
  import { marked } from 'marked'

  let {
    content,
    fontSize,
    onRead,
    onProgress,
  }: {
    content: string
    fontSize: number
    onRead: () => void
    onProgress: (pct: number) => void
  } = $props()

  let didReport = false
  let scrollEl: HTMLDivElement | undefined = $state()
  const html = $derived(marked.parse(content) as string)

  $effect(() => {
    void content
    didReport = false
  })

  $effect(() => {
    const el = scrollEl
    if (!el) return
    function onScroll() {
      const el2 = el as HTMLDivElement
      const pct = el2.scrollHeight <= el2.clientHeight ? 1 : el2.scrollTop / (el2.scrollHeight - el2.clientHeight)
      onProgress(Math.min(1, pct))
      if (didReport) return
      if (el2.scrollTop + el2.clientHeight >= el2.scrollHeight - 100) {
        didReport = true
        onRead()
      }
    }
    el.addEventListener('scroll', onScroll, { passive: true })
    return () => el.removeEventListener('scroll', onScroll)
  })
</script>

<div bind:this={scrollEl} class="novel-scroll">
  <div class="mx-auto max-w-2xl px-6 py-10">
    <div class="novel-prose" style={`font-size: ${fontSize}px`}>
      {@html html}
    </div>
  </div>
</div>
