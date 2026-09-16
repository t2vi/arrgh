import { render } from '@testing-library/svelte'
import { afterEach, describe, expect, it, vi } from 'vitest'
import ScrollReader from './ScrollReader.svelte'

// Mocks getBoundingClientRect so onScroll's page-detection loop (ScrollReader.svelte's
// onScroll) can run in jsdom, which does no real layout. Page images are modeled as
// stacked 500px-tall blocks; the container itself never moves (top: 0) — only its
// scrollTop, read live off the element, shifts where each image appears.
function mockPageLayout() {
  vi.spyOn(Element.prototype, 'getBoundingClientRect').mockImplementation(function (this: Element) {
    const base = { bottom: 0, left: 0, right: 0, width: 0, x: 0, y: 0, toJSON: () => ({}) }
    if (this.hasAttribute('data-page')) {
      const scrollDiv = this.closest('.overflow-y-auto') as HTMLDivElement | null
      const page = Number(this.getAttribute('data-page'))
      return { ...base, top: page * 500 - (scrollDiv?.scrollTop ?? 0), height: 500 } as DOMRect
    }
    return { ...base, top: 0, height: 800 } as DOMRect
  })
}

afterEach(() => {
  vi.restoreAllMocks()
})

describe('ScrollReader — page tracking does not fight the user scroll', () => {
  it('applies initialPage once on open, then leaves scrollTop alone on later page-seen updates', () => {
    const onPageSeen = vi.fn()
    const { container, rerender } = render(ScrollReader, {
      chapterId: 'ch1',
      total: 20,
      initialPage: 5,
      onPageSeen,
      onLastPageFailed: vi.fn(),
    })

    const scrollDiv = container.querySelector('.overflow-y-auto') as HTMLDivElement
    Object.defineProperty(scrollDiv, 'clientHeight', { value: 800, configurable: true })

    // Opening the chapter mid-way (FR-002/SC-002) still jumps to the saved page once.
    expect(scrollDiv.scrollTop).toBe(2500)

    mockPageLayout()

    // User scrolls further on their own — not a multiple of 500, so a buggy re-snap
    // to `current * 500` is distinguishable from the user's real position.
    scrollDiv.scrollTop = 6789
    scrollDiv.dispatchEvent(new Event('scroll'))

    expect(onPageSeen).toHaveBeenCalledWith(14)

    // Simulate the real round-trip: the parent writes the seen page back into
    // `initialPage` (Reader.svelte's `initialPage={store.page}` / `onPageSeen={(p) =>
    // store.goTo(p)}`). This must NOT force scrollTop back to `14 * 500` — that's the
    // feedback loop that trapped the reader on page 1 (GH #172).
    rerender({
      chapterId: 'ch1',
      total: 20,
      initialPage: 14,
      onPageSeen,
      onLastPageFailed: vi.fn(),
    })

    expect(scrollDiv.scrollTop).toBe(6789)
  })
})
