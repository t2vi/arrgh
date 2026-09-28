import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'

vi.mock('./api', () => ({
  api: {
    getChapter: vi.fn(),
    getProgress: vi.fn(),
    getSettings: vi.fn(),
    getChapterText: vi.fn(),
    getTitle: vi.fn(),
    listChapters: vi.fn(),
    updateProgress: vi.fn(),
    downloadChapter: vi.fn(),
  },
}))

import { ReaderStore } from './reader.svelte'
import { api } from './api'
import type { Chapter } from './types'

function chapter(overrides: Partial<Chapter> = {}): Chapter {
  return {
    id: 'ch1', title_id: 'title1', title: null, number: 1, volume: null,
    local_path: null, page_count: 10, downloaded: true, has_sources: true,
    chapter_format: 'image', created_at: '2026-01-01T00:00:00Z',
    ...overrides,
  }
}

beforeEach(() => {
  vi.clearAllMocks()
  vi.mocked(api.getProgress).mockResolvedValue({ chapter_id: 'ch1', current_page: 0, completed: false } as never)
  vi.mocked(api.getSettings).mockResolvedValue({ reader_mode: 'paged' } as never)
  vi.mocked(api.getTitle).mockResolvedValue({ id: 'title1' } as never)
  vi.mocked(api.listChapters).mockResolvedValue([])
  vi.mocked(api.downloadChapter).mockResolvedValue(undefined as never)
  vi.mocked(api.updateProgress).mockResolvedValue(undefined as never)
})

afterEach(() => {
  vi.useRealTimers()
})

// ReaderStore's chapter-load and poll effects only run inside a Svelte effect
// root — $effect.root() gives it one outside of component rendering, same as
// queue.svelte.test.ts's createStore() pattern.
function createStore() {
  let store!: ReaderStore
  const cleanup = $effect.root(() => {
    store = new ReaderStore()
  })
  return { store, cleanup }
}

// spec: 013/FR-001, 013/FR-002, 013/FR-003, 013/FR-005, 015/FR-005
describe('ReaderStore — triggers a download instead of leaving a blank chapter', () => {
  it('triggers a download and sets chapterDownloading for an undownloaded, sourced chapter', async () => {
    vi.mocked(api.getChapter).mockResolvedValue(chapter({ downloaded: false, has_sources: true }))
    const { store, cleanup } = createStore()
    store.load('ch1')

    await vi.waitFor(() => expect(store.chapterDownloading).toBe(true))
    expect(api.downloadChapter).toHaveBeenCalledWith('ch1')
    expect(api.downloadChapter).toHaveBeenCalledTimes(1)
    cleanup()
  })

  it('clears chapterDownloading once a poll reports the chapter downloaded', async () => {
    vi.useFakeTimers()
    vi.mocked(api.getChapter)
      .mockResolvedValueOnce(chapter({ downloaded: false, has_sources: true }))
      .mockResolvedValue(chapter({ downloaded: true, has_sources: true }))
    const { store, cleanup } = createStore()
    store.load('ch1')

    await vi.advanceTimersByTimeAsync(0)
    expect(store.chapterDownloading).toBe(true)

    await vi.advanceTimersByTimeAsync(2000)
    expect(store.chapterDownloading).toBe(false)
    expect(store.chapterUnavailable).toBe(false)
    expect(store.chapter?.downloaded).toBe(true)
    cleanup()
  })

  // spec: 013/FR-004, 015/FR-007
  it('never triggers a download for a chapter with no source, and marks it unavailable', async () => {
    vi.mocked(api.getChapter).mockResolvedValue(chapter({ downloaded: false, has_sources: false }))
    const { store, cleanup } = createStore()
    store.load('ch1')

    await vi.waitFor(() => expect(store.chapterUnavailable).toBe(true))
    expect(api.downloadChapter).not.toHaveBeenCalled()
    expect(store.chapterDownloading).toBe(false)
    cleanup()
  })

  it('does not trigger a download for an already-downloaded chapter', async () => {
    vi.mocked(api.getChapter).mockResolvedValue(chapter({ downloaded: true, has_sources: true }))
    const { store, cleanup } = createStore()
    store.load('ch1')

    await vi.waitFor(() => expect(store.chapter).toBeDefined())
    expect(api.downloadChapter).not.toHaveBeenCalled()
    expect(store.chapterDownloading).toBe(false)
    cleanup()
  })
})

// spec: 015/FR-002
describe('ReaderStore — paged navigation', () => {
  it('goTo clamps to the known page range and persists the new position', async () => {
    vi.mocked(api.getChapter).mockResolvedValue(chapter({ page_count: 10 }))
    const { store, cleanup } = createStore()
    store.load('ch1')
    await vi.waitFor(() => expect(store.chapter).toBeDefined())

    store.goTo(20)
    expect(store.page).toBe(9)
    expect(api.updateProgress).toHaveBeenCalledWith('ch1', 9, true)

    store.goTo(-5)
    expect(store.page).toBe(0)
    expect(api.updateProgress).toHaveBeenCalledWith('ch1', 0, false)
    cleanup()
  })

  it('ArrowRight/ArrowLeft advance and retreat one page in paged mode', async () => {
    vi.mocked(api.getChapter).mockResolvedValue(chapter({ page_count: 10 }))
    vi.mocked(api.getSettings).mockResolvedValue({ reader_mode: 'paged' } as never)
    const { store, cleanup } = createStore()
    store.load('ch1')
    await vi.waitFor(() => expect(store.chapter).toBeDefined())
    expect(store.effectiveMode).toBe('paged')

    window.dispatchEvent(new KeyboardEvent('keydown', { key: 'ArrowRight' }))
    expect(store.page).toBe(1)
    window.dispatchEvent(new KeyboardEvent('keydown', { key: 'ArrowLeft' }))
    expect(store.page).toBe(0)
    cleanup()
  })
})

// spec: 014/FR-001, 014/FR-003, 015/FR-001
describe('ReaderStore — effective reader mode fallback chain', () => {
  it('falls back to scroll when no per-title override and no global setting is saved', async () => {
    vi.mocked(api.getChapter).mockResolvedValue(chapter())
    vi.mocked(api.getTitle).mockResolvedValue({ id: 'title1', reader_mode: null } as never)
    vi.mocked(api.getSettings).mockResolvedValue({ reader_mode: null } as never)
    const { store, cleanup } = createStore()
    store.load('ch1')

    await vi.waitFor(() => expect(store.manga).toBeDefined())
    expect(store.effectiveMode).toBe('scroll')
    cleanup()
  })

  it('the global default is used when no per-title override exists', async () => {
    vi.mocked(api.getChapter).mockResolvedValue(chapter())
    vi.mocked(api.getTitle).mockResolvedValue({ id: 'title1', reader_mode: null } as never)
    vi.mocked(api.getSettings).mockResolvedValue({ reader_mode: 'paged' } as never)
    const { store, cleanup } = createStore()
    store.load('ch1')

    await vi.waitFor(() => expect(store.manga).toBeDefined())
    expect(store.effectiveMode).toBe('paged')
    cleanup()
  })

  it('a per-title override wins over the global default', async () => {
    vi.mocked(api.getChapter).mockResolvedValue(chapter())
    vi.mocked(api.getTitle).mockResolvedValue({ id: 'title1', reader_mode: 'paged' } as never)
    vi.mocked(api.getSettings).mockResolvedValue({ reader_mode: 'scroll' } as never)
    const { store, cleanup } = createStore()
    store.load('ch1')

    await vi.waitFor(() => expect(store.manga).toBeDefined())
    expect(store.effectiveMode).toBe('paged')
    cleanup()
  })

  it('an in-session toggle overrides the resolved mode and flips back without a reload', async () => {
    vi.mocked(api.getChapter).mockResolvedValue(chapter())
    vi.mocked(api.getTitle).mockResolvedValue({ id: 'title1', reader_mode: 'paged' } as never)
    const { store, cleanup } = createStore()
    store.load('ch1')
    await vi.waitFor(() => expect(store.manga).toBeDefined())
    expect(store.effectiveMode).toBe('paged')

    store.toggleMode()
    expect(store.effectiveMode).toBe('scroll')
    store.toggleMode()
    expect(store.effectiveMode).toBe('paged')
    cleanup()
  })
})
