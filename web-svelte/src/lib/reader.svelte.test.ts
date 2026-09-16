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
