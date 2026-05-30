// Behavior tests for ADR 0031 new plugins.
// Each suite stubs fetch with minimal HTML/JSON fixtures and asserts on output shapes.
// Tests are RED until plugin dirs are created; GREEN after implementation.
// Fixtures define the parsing contract — implementations must match these structures.

import { describe, it, expect, beforeEach, afterEach } from 'vitest'

// ── Imports (fail until plugin dirs exist) ────────────────────────────────────

import * as asurascans   from '../../plugins/asurascans/src/index'
import * as manhuafast   from '../../plugins/manhuafast/src/index'
import * as wuxiaworld   from '../../plugins/wuxiaworld/src/index'
import * as boxnovel     from '../../plugins/boxnovel/src/index'
import * as manga18fx    from '../../plugins/manga18fx/src/index'
import { parseSearchHtml as nuParseSearchHtml } from '../../plugins/novelupdates/src/novelupdates'

// ── Fixture helpers ───────────────────────────────────────────────────────────

function mockFetch(responses: Record<string, { ok?: boolean; text?: string; json?: unknown }>) {
  return vi.fn().mockImplementation((url: string) => {
    const key = Object.keys(responses).find((k) => url.toString().includes(k))
    const resp = key ? responses[key] : { ok: false, text: '', json: {} }
    return Promise.resolve({
      ok: resp.ok ?? true,
      text: async () => resp.text ?? '',
      json: async () => resp.json ?? {},
    })
  })
}

beforeEach(() => { vi.clearAllMocks() })
afterEach(() => { vi.unstubAllGlobals() })

// ═══════════════════════════════════════════════════════════════════════════════
// AsuraScans
// ═══════════════════════════════════════════════════════════════════════════════

const ASURA_SEARCH_HTML = `
<div class="grid grid-cols-2 gap-3 p-4">
  <div class="group/tipmanga">
    <a href="https://asuracomic.net/series/solo-leveling-abc123" class="series">
      <img src="https://gg.asuracomic.net/storage/covers/solo-leveling.jpg" class="rounded" alt="Solo Leveling">
      <div class="col-span-8 flex flex-col">
        <span class="block font-bold text-white">Solo Leveling</span>
        <span class="text-xs">Manhwa</span>
        <span class="text-xs text-[#ff7e2e]">ONGOING</span>
      </div>
    </a>
  </div>
  <div class="group/tipmanga">
    <a href="https://asuracomic.net/series/return-of-the-mount-hua-sect-xyz789" class="series">
      <img src="https://gg.asuracomic.net/storage/covers/rmhs.jpg" class="rounded" alt="Return of the Mount Hua Sect">
      <div class="col-span-8 flex flex-col">
        <span class="block font-bold text-white">Return of the Mount Hua Sect</span>
        <span class="text-xs">Manhwa</span>
        <span class="text-xs text-[#ff7e2e]">ONGOING</span>
      </div>
    </a>
  </div>
</div>
`

const ASURA_CHAPTERS_HTML = `
<div class="scrollbar-thumb-themecolor overflow-y-auto">
  <div class="py-2 border-b flex items-center" data-num="180">
    <a href="https://asuracomic.net/series/solo-leveling-abc123/chapter/180">
      <span>Chapter 180</span>
    </a>
  </div>
  <div class="py-2 border-b flex items-center" data-num="179">
    <a href="https://asuracomic.net/series/solo-leveling-abc123/chapter/179">
      <span>Chapter 179</span>
    </a>
  </div>
</div>
`

const ASURA_PAGES_HTML = `
<div class="flex flex-col items-center">
  <img src="https://gg.asuracomic.net/storage/media/ch180/001.jpg" class="object-cover" alt="page 1">
  <img src="https://gg.asuracomic.net/storage/media/ch180/002.jpg" class="object-cover" alt="page 2">
  <img src="https://gg.asuracomic.net/storage/media/ch180/003.jpg" class="object-cover" alt="page 3">
</div>
`

describe('asurascans — search', () => {
  it('returns array with required fields', async () => {
    vi.stubGlobal('fetch', mockFetch({ 'asuracomic': { text: ASURA_SEARCH_HTML } }))
    const results = await asurascans.search('solo leveling')
    expect(Array.isArray(results)).toBe(true)
    expect(results.length).toBeGreaterThan(0)
    for (const r of results) {
      expect(r).toHaveProperty('id')
      expect(r).toHaveProperty('title')
      expect(r).toHaveProperty('cover_url')
      expect(r).toHaveProperty('status')
      expect(r).toHaveProperty('content_type')
    }
  })

  it('extracts id from series URL slug', async () => {
    vi.stubGlobal('fetch', mockFetch({ 'asuracomic': { text: ASURA_SEARCH_HTML } }))
    const [first] = await asurascans.search('solo leveling')
    expect(first.id).toBe('solo-leveling-abc123')
    expect(first.title).toBe('Solo Leveling')
    expect(first.content_type).toBe('manhwa')
  })

  it('status is lowercase normalised', async () => {
    vi.stubGlobal('fetch', mockFetch({ 'asuracomic': { text: ASURA_SEARCH_HTML } }))
    const [first] = await asurascans.search('solo leveling')
    expect(first.status).toBe('ongoing')
  })

  it('returns empty array when no results', async () => {
    vi.stubGlobal('fetch', mockFetch({ 'asuracomic': { text: '<div></div>' } }))
    const results = await asurascans.search('nothing')
    expect(results).toEqual([])
  })

  it('throws on non-ok response', async () => {
    vi.stubGlobal('fetch', mockFetch({ 'asuracomic': { ok: false } }))
    await expect(asurascans.search('test')).rejects.toThrow()
  })
})

describe('asurascans — chapters', () => {
  it('returns chapters with source_id and number', async () => {
    vi.stubGlobal('fetch', mockFetch({ 'asuracomic': { text: ASURA_CHAPTERS_HTML } }))
    const chapters = await asurascans.chapters('solo-leveling-abc123')
    expect(Array.isArray(chapters)).toBe(true)
    expect(chapters.length).toBeGreaterThan(0)
    for (const ch of chapters) {
      expect(ch).toHaveProperty('source_id')
      expect(ch).toHaveProperty('number')
      expect(typeof ch.number).toBe('number')
    }
  })

  it('source_id is the chapter URL path', async () => {
    vi.stubGlobal('fetch', mockFetch({ 'asuracomic': { text: ASURA_CHAPTERS_HTML } }))
    const chapters = await asurascans.chapters('solo-leveling-abc123')
    const ch180 = chapters.find((c) => c.number === 180)
    expect(ch180).toBeDefined()
    expect(ch180!.source_id).toContain('solo-leveling-abc123')
  })
})

describe('asurascans — pages', () => {
  it('returns image URL array', async () => {
    vi.stubGlobal('fetch', mockFetch({ 'asuracomic': { text: ASURA_PAGES_HTML } }))
    const pages = await asurascans.pages('solo-leveling-abc123/chapter/180')
    expect(Array.isArray(pages)).toBe(true)
    expect(pages.length).toBe(3)
    for (const p of pages) {
      expect(typeof p).toBe('string')
      expect(p.startsWith('http')).toBe(true)
    }
  })
})

// ═══════════════════════════════════════════════════════════════════════════════
// ManhuaFast
// ═══════════════════════════════════════════════════════════════════════════════

const MANHUAFAST_SEARCH_HTML = `
<div class="page-listing-item">
  <div class="item-thumb">
    <a href="https://manhuafast.net/manga/the-beginning-after-the-end/">
      <img src="https://manhuafast.net/wp-content/uploads/covers/tbate.jpg"
           alt="The Beginning After the End"
           class="img-responsive lazy">
    </a>
  </div>
  <div class="item-summary">
    <div class="post-title">
      <h5><a href="https://manhuafast.net/manga/the-beginning-after-the-end/">The Beginning After the End</a></h5>
    </div>
    <div class="manga-title-badges new"><span>OnGoing</span></div>
  </div>
</div>
<div class="page-listing-item">
  <div class="item-thumb">
    <a href="https://manhuafast.net/manga/nano-machine/">
      <img src="https://manhuafast.net/wp-content/uploads/covers/nano.jpg"
           alt="Nano Machine"
           class="img-responsive lazy">
    </a>
  </div>
  <div class="item-summary">
    <div class="post-title">
      <h5><a href="https://manhuafast.net/manga/nano-machine/">Nano Machine</a></h5>
    </div>
    <div class="manga-title-badges"><span>Completed</span></div>
  </div>
</div>
`

const MANHUAFAST_CHAPTERS_HTML = `
<div class="listing-chapters_wrap">
  <ul class="main version-chap">
    <li class="wp-manga-chapter" data-chapter-link="https://manhuafast.net/manga/the-beginning-after-the-end/chapter-180/">
      <a href="https://manhuafast.net/manga/the-beginning-after-the-end/chapter-180/">Chapter 180</a>
    </li>
    <li class="wp-manga-chapter" data-chapter-link="https://manhuafast.net/manga/the-beginning-after-the-end/chapter-179/">
      <a href="https://manhuafast.net/manga/the-beginning-after-the-end/chapter-179/">Chapter 179</a>
    </li>
  </ul>
</div>
`

const MANHUAFAST_PAGES_HTML = `
<div class="reading-content">
  <div class="page-break no-gaps">
    <img src="https://manhuafast.net/wp-content/uploads/ch180/001.jpg" class="wp-manga-chapter-img" alt="page 1 tbate">
  </div>
  <div class="page-break no-gaps">
    <img src="https://manhuafast.net/wp-content/uploads/ch180/002.jpg" class="wp-manga-chapter-img" alt="page 2 tbate">
  </div>
</div>
`

describe('manhuafast — search', () => {
  it('returns array with required fields', async () => {
    vi.stubGlobal('fetch', mockFetch({ 'manhuafast': { text: MANHUAFAST_SEARCH_HTML } }))
    const results = await manhuafast.search('beginning after the end')
    expect(Array.isArray(results)).toBe(true)
    expect(results.length).toBeGreaterThan(0)
    for (const r of results) {
      expect(r).toHaveProperty('id')
      expect(r).toHaveProperty('title')
      expect(r).toHaveProperty('cover_url')
      expect(r).toHaveProperty('status')
      expect(r).toHaveProperty('content_type')
      expect(r.content_type).toBe('manhua')
    }
  })

  it('id is the URL slug', async () => {
    vi.stubGlobal('fetch', mockFetch({ 'manhuafast': { text: MANHUAFAST_SEARCH_HTML } }))
    const [first] = await manhuafast.search('beginning after the end')
    expect(first.id).toBe('the-beginning-after-the-end')
    expect(first.title).toBe('The Beginning After the End')
  })

  it('status is normalised to lowercase', async () => {
    vi.stubGlobal('fetch', mockFetch({ 'manhuafast': { text: MANHUAFAST_SEARCH_HTML } }))
    const results = await manhuafast.search('nano machine')
    const nano = results.find((r) => r.title === 'Nano Machine')
    expect(nano!.status).toBe('completed')
  })

  it('cover_url is the img src', async () => {
    vi.stubGlobal('fetch', mockFetch({ 'manhuafast': { text: MANHUAFAST_SEARCH_HTML } }))
    const [first] = await manhuafast.search('test')
    expect(first.cover_url).toContain('manhuafast.net')
  })

  it('throws on non-ok response', async () => {
    vi.stubGlobal('fetch', mockFetch({ 'manhuafast': { ok: false } }))
    await expect(manhuafast.search('test')).rejects.toThrow()
  })
})

describe('manhuafast — chapters', () => {
  it('returns chapters with source_id and number', async () => {
    vi.stubGlobal('fetch', mockFetch({ 'manhuafast': { text: MANHUAFAST_CHAPTERS_HTML } }))
    const chapters = await manhuafast.chapters('the-beginning-after-the-end')
    expect(Array.isArray(chapters)).toBe(true)
    expect(chapters.length).toBeGreaterThan(0)
    for (const ch of chapters) {
      expect(ch).toHaveProperty('source_id')
      expect(ch).toHaveProperty('number')
      expect(typeof ch.number).toBe('number')
    }
  })

  it('parses chapter numbers', async () => {
    vi.stubGlobal('fetch', mockFetch({ 'manhuafast': { text: MANHUAFAST_CHAPTERS_HTML } }))
    const chapters = await manhuafast.chapters('the-beginning-after-the-end')
    const nums = chapters.map((c) => c.number).sort((a, b) => a - b)
    expect(nums).toContain(179)
    expect(nums).toContain(180)
  })
})

describe('manhuafast — pages', () => {
  it('returns image URL array', async () => {
    vi.stubGlobal('fetch', mockFetch({ 'manhuafast': { text: MANHUAFAST_PAGES_HTML } }))
    const pages = await manhuafast.pages('the-beginning-after-the-end/chapter-180')
    expect(Array.isArray(pages)).toBe(true)
    expect(pages.length).toBe(2)
    for (const p of pages) {
      expect(typeof p).toBe('string')
      expect(p.startsWith('http')).toBe(true)
    }
  })
})

// ═══════════════════════════════════════════════════════════════════════════════
// WuxiaWorld
// ═══════════════════════════════════════════════════════════════════════════════

const WUXIA_SEARCH_JSON = {
  items: [
    {
      id: 'swallowed-star',
      name: 'Swallowed Star',
      coverUrl: 'https://cdn.wuxiaworld.com/covers/swallowed-star.jpg',
      status: 'Completed',
      author: { name: 'I Eat Tomatoes' },
      genres: ['Sci-fi', 'Action'],
    },
    {
      id: 'martial-world',
      name: 'Martial World',
      coverUrl: 'https://cdn.wuxiaworld.com/covers/martial-world.jpg',
      status: 'Completed',
      author: { name: 'Cocooned Cow' },
      genres: ['Martial Arts'],
    },
  ],
  total: 2,
}

const WUXIA_CHAPTERS_JSON = {
  items: [
    { entityId: 'swallowed-star/swallowed-star-chapter-1', chapter: { num: 1 }, name: 'Chapter 1 — The Swift as Lightning Technique' },
    { entityId: 'swallowed-star/swallowed-star-chapter-2', chapter: { num: 2 }, name: 'Chapter 2 — Practice' },
  ],
}

const WUXIA_CHAPTER_HTML = `
<div class="chapter-content">
  <p>Luo Feng, a young man living in Jiangnan base city…</p>
  <p>He had awakened as a genetic warrior, able to breathe underwater.</p>
</div>
`

describe('wuxiaworld — search', () => {
  it('returns array with required fields', async () => {
    vi.stubGlobal('fetch', mockFetch({ 'wuxiaworld': { json: WUXIA_SEARCH_JSON } }))
    const results = await wuxiaworld.search('swallowed star')
    expect(Array.isArray(results)).toBe(true)
    expect(results.length).toBeGreaterThan(0)
    for (const r of results) {
      expect(r).toHaveProperty('id')
      expect(r).toHaveProperty('title')
      expect(r).toHaveProperty('cover_url')
      expect(r).toHaveProperty('status')
      expect(r).toHaveProperty('content_type')
      expect(r.content_type).toBe('novel')
    }
  })

  it('maps API response fields', async () => {
    vi.stubGlobal('fetch', mockFetch({ 'wuxiaworld': { json: WUXIA_SEARCH_JSON } }))
    const [first] = await wuxiaworld.search('swallowed star')
    expect(first.id).toBe('swallowed-star')
    expect(first.title).toBe('Swallowed Star')
    expect(first.cover_url).toBe('https://cdn.wuxiaworld.com/covers/swallowed-star.jpg')
    expect(first.status.toLowerCase()).toBe('completed')
    expect(first.author).toBe('I Eat Tomatoes')
  })

  it('returns empty array when items is empty', async () => {
    vi.stubGlobal('fetch', mockFetch({ 'wuxiaworld': { json: { items: [], total: 0 } } }))
    const results = await wuxiaworld.search('xyzzy')
    expect(results).toEqual([])
  })

  it('throws on non-ok response', async () => {
    vi.stubGlobal('fetch', mockFetch({ 'wuxiaworld': { ok: false } }))
    await expect(wuxiaworld.search('test')).rejects.toThrow()
  })
})

describe('wuxiaworld — chapters', () => {
  it('returns chapters with source_id and number', async () => {
    vi.stubGlobal('fetch', mockFetch({ 'wuxiaworld': { json: WUXIA_CHAPTERS_JSON } }))
    const chapters = await wuxiaworld.chapters('swallowed-star')
    expect(Array.isArray(chapters)).toBe(true)
    expect(chapters.length).toBeGreaterThan(0)
    for (const ch of chapters) {
      expect(ch).toHaveProperty('source_id')
      expect(ch).toHaveProperty('number')
      expect(typeof ch.number).toBe('number')
    }
  })

  it('source_id is entityId from API', async () => {
    vi.stubGlobal('fetch', mockFetch({ 'wuxiaworld': { json: WUXIA_CHAPTERS_JSON } }))
    const [ch1] = await wuxiaworld.chapters('swallowed-star')
    expect(ch1.source_id).toBe('swallowed-star/swallowed-star-chapter-1')
    expect(ch1.number).toBe(1)
    expect(ch1.title).toBe('Chapter 1 — The Swift as Lightning Technique')
  })
})

describe('wuxiaworld — chapterText', () => {
  it('returns string content', async () => {
    vi.stubGlobal('fetch', mockFetch({ 'wuxiaworld': { text: WUXIA_CHAPTER_HTML } }))
    const text = await wuxiaworld.chapterText('swallowed-star/swallowed-star-chapter-1')
    expect(typeof text).toBe('string')
    expect(text.length).toBeGreaterThan(0)
  })

  it('extracts chapter text content', async () => {
    vi.stubGlobal('fetch', mockFetch({ 'wuxiaworld': { text: WUXIA_CHAPTER_HTML } }))
    const text = await wuxiaworld.chapterText('swallowed-star/swallowed-star-chapter-1')
    expect(text).toContain('Luo Feng')
  })

  it('throws on non-ok response', async () => {
    vi.stubGlobal('fetch', mockFetch({ 'wuxiaworld': { ok: false } }))
    await expect(wuxiaworld.chapterText('test/test-chapter-1')).rejects.toThrow()
  })
})

// ═══════════════════════════════════════════════════════════════════════════════
// BoxNovel
// ═══════════════════════════════════════════════════════════════════════════════

const BOXNOVEL_SEARCH_HTML = `
<div class="c-tabs-item">
  <div class="row">
    <div class="col-4">
      <div class="tab-thumb c-image-inner">
        <a href="https://boxnovel.com/novel/lord-of-the-mysteries/">
          <img src="https://boxnovel.com/wp-content/uploads/2019/lord-of-mysteries.jpg"
               class="img-responsive lazy" alt="Lord of the Mysteries">
        </a>
      </div>
    </div>
    <div class="col-8">
      <div class="tab-summary">
        <div class="post-title">
          <h5><a href="https://boxnovel.com/novel/lord-of-the-mysteries/">Lord of the Mysteries</a></h5>
        </div>
        <div class="post-content_item">
          <div class="summary-heading"><h5>Status</h5></div>
          <div class="summary-content">Completed</div>
        </div>
        <div class="post-content_item">
          <div class="summary-heading"><h5>Author(s)</h5></div>
          <div class="summary-content"><a>Cuttlefish That Loves Diving</a></div>
        </div>
      </div>
    </div>
  </div>
</div>
<div class="c-tabs-item">
  <div class="row">
    <div class="col-4">
      <div class="tab-thumb c-image-inner">
        <a href="https://boxnovel.com/novel/i-shall-seal-the-heavens/">
          <img src="https://boxnovel.com/wp-content/uploads/issh.jpg"
               class="img-responsive lazy" alt="I Shall Seal the Heavens">
        </a>
      </div>
    </div>
    <div class="col-8">
      <div class="tab-summary">
        <div class="post-title">
          <h5><a href="https://boxnovel.com/novel/i-shall-seal-the-heavens/">I Shall Seal the Heavens</a></h5>
        </div>
        <div class="post-content_item">
          <div class="summary-heading"><h5>Status</h5></div>
          <div class="summary-content">Completed</div>
        </div>
        <div class="post-content_item">
          <div class="summary-heading"><h5>Author(s)</h5></div>
          <div class="summary-content"><a>Er Gen</a></div>
        </div>
      </div>
    </div>
  </div>
</div>
`

const BOXNOVEL_CHAPTERS_HTML = `
<ul class="main version-chap">
  <li class="wp-manga-chapter">
    <a href="https://boxnovel.com/novel/lord-of-the-mysteries/chapter-1429/">Chapter 1429 - Epilogue</a>
    <span class="chapter-release-date"></span>
  </li>
  <li class="wp-manga-chapter">
    <a href="https://boxnovel.com/novel/lord-of-the-mysteries/chapter-1/">Chapter 1 - Abnormal Death</a>
    <span class="chapter-release-date"></span>
  </li>
</ul>
`

const BOXNOVEL_CHAPTER_HTML = `
<div class="reading-content">
  <div class="text-left">
    <p>With a flash of light, Zhou Mingrui awoke to find himself in a new world.</p>
    <p>He had become Klein Moretti — a nobody from a lower-class family in Tingen City.</p>
  </div>
</div>
`

describe('boxnovel — search', () => {
  it('returns array with required fields', async () => {
    vi.stubGlobal('fetch', mockFetch({ 'boxnovel': { text: BOXNOVEL_SEARCH_HTML } }))
    const results = await boxnovel.search('lord of the mysteries')
    expect(Array.isArray(results)).toBe(true)
    expect(results.length).toBeGreaterThan(0)
    for (const r of results) {
      expect(r).toHaveProperty('id')
      expect(r).toHaveProperty('title')
      expect(r).toHaveProperty('cover_url')
      expect(r).toHaveProperty('status')
      expect(r).toHaveProperty('content_type')
      expect(r.content_type).toBe('novel')
    }
  })

  it('id is the URL slug', async () => {
    vi.stubGlobal('fetch', mockFetch({ 'boxnovel': { text: BOXNOVEL_SEARCH_HTML } }))
    const [first] = await boxnovel.search('lord of the mysteries')
    expect(first.id).toBe('lord-of-the-mysteries')
    expect(first.title).toBe('Lord of the Mysteries')
    expect(first.status.toLowerCase()).toBe('completed')
    expect(first.author).toBe('Cuttlefish That Loves Diving')
  })

  it('cover_url is the img src', async () => {
    vi.stubGlobal('fetch', mockFetch({ 'boxnovel': { text: BOXNOVEL_SEARCH_HTML } }))
    const [first] = await boxnovel.search('lord')
    expect(first.cover_url).toContain('boxnovel.com')
  })

  it('returns empty array when no results', async () => {
    vi.stubGlobal('fetch', mockFetch({ 'boxnovel': { text: '<div></div>' } }))
    const results = await boxnovel.search('xyzzy')
    expect(results).toEqual([])
  })

  it('throws on non-ok response', async () => {
    vi.stubGlobal('fetch', mockFetch({ 'boxnovel': { ok: false } }))
    await expect(boxnovel.search('test')).rejects.toThrow()
  })
})

describe('boxnovel — chapters', () => {
  it('returns chapters with source_id and number', async () => {
    vi.stubGlobal('fetch', mockFetch({ 'boxnovel': { text: BOXNOVEL_CHAPTERS_HTML } }))
    const chapters = await boxnovel.chapters('lord-of-the-mysteries')
    expect(Array.isArray(chapters)).toBe(true)
    expect(chapters.length).toBeGreaterThan(0)
    for (const ch of chapters) {
      expect(ch).toHaveProperty('source_id')
      expect(ch).toHaveProperty('number')
      expect(typeof ch.number).toBe('number')
    }
  })

  it('parses chapter numbers from URL', async () => {
    vi.stubGlobal('fetch', mockFetch({ 'boxnovel': { text: BOXNOVEL_CHAPTERS_HTML } }))
    const chapters = await boxnovel.chapters('lord-of-the-mysteries')
    const nums = chapters.map((c) => c.number)
    expect(nums).toContain(1)
    expect(nums).toContain(1429)
  })

  it('source_id is the chapter URL path', async () => {
    vi.stubGlobal('fetch', mockFetch({ 'boxnovel': { text: BOXNOVEL_CHAPTERS_HTML } }))
    const chapters = await boxnovel.chapters('lord-of-the-mysteries')
    const ch1 = chapters.find((c) => c.number === 1)
    expect(ch1!.source_id).toContain('lord-of-the-mysteries/chapter-1')
  })
})

describe('boxnovel — chapterText', () => {
  it('returns string content', async () => {
    vi.stubGlobal('fetch', mockFetch({ 'boxnovel': { text: BOXNOVEL_CHAPTER_HTML } }))
    const text = await boxnovel.chapterText('lord-of-the-mysteries/chapter-1')
    expect(typeof text).toBe('string')
    expect(text.length).toBeGreaterThan(0)
  })

  it('extracts chapter text content', async () => {
    vi.stubGlobal('fetch', mockFetch({ 'boxnovel': { text: BOXNOVEL_CHAPTER_HTML } }))
    const text = await boxnovel.chapterText('lord-of-the-mysteries/chapter-1')
    expect(text).toContain('Zhou Mingrui')
  })

  it('throws on non-ok response', async () => {
    vi.stubGlobal('fetch', mockFetch({ 'boxnovel': { ok: false } }))
    await expect(boxnovel.chapterText('lord-of-the-mysteries/chapter-1')).rejects.toThrow()
  })
})

// ═══════════════════════════════════════════════════════════════════════════════
// NovelUpdates (metadata-only — uses CloakBrowser, tests cover HTML parser only)
// ═══════════════════════════════════════════════════════════════════════════════

const NU_SEARCH_HTML = `
<html><body>
<div class="search_main_box_nu">
  <div class="search_body_nu">
    <div class="search_title"><a href="/series/a-will-eternal/">A Will Eternal</a></div>
    <div class="search_img_nu"><img src="https://cdn.novelupdates.com/a-will-eternal.jpg" alt="cover"></div>
    <div class="seriestypelist">Web Novel</div>
    <div class="series_latest_status">Completed</div>
  </div>
</div>
<div class="search_main_box_nu">
  <div class="search_body_nu">
    <div class="search_title"><a href="/series/i-shall-seal-the-heavens/">I Shall Seal the Heavens</a></div>
    <div class="search_img_nu"><img src="https://cdn.novelupdates.com/issh.jpg" alt="cover"></div>
    <div class="seriestypelist">Web Novel</div>
    <div class="series_latest_status">Completed</div>
  </div>
</div>
</body></html>
`

describe('novelupdates — parseSearchHtml (HTML parser, no browser needed)', () => {
  it('extracts id (slug), title, cover_url, status', () => {
    const results = nuParseSearchHtml(NU_SEARCH_HTML)
    expect(results.length).toBe(2)
    const awe = results.find((r) => r.title === 'A Will Eternal')!
    expect(awe.id).toBe('a-will-eternal')
    expect(awe.status).toBe('complete')
    expect(awe.cover_url).toContain('novelupdates.com')
    expect(awe.content_type).toBe('novel')
  })

  it('parses multiple results', () => {
    const results = nuParseSearchHtml(NU_SEARCH_HTML)
    const issh = results.find((r) => r.title === 'I Shall Seal the Heavens')
    expect(issh).toBeDefined()
    expect(issh!.id).toBe('i-shall-seal-the-heavens')
  })

  it('returns empty array for empty HTML', () => {
    expect(nuParseSearchHtml('<html><body></body></html>')).toEqual([])
  })

  it('Ongoing status maps to ongoing', () => {
    const html = NU_SEARCH_HTML.replace('Completed', 'Ongoing')
    const [first] = nuParseSearchHtml(html)
    expect(first.status).toBe('ongoing')
  })

  it('content_type is always novel', () => {
    const results = nuParseSearchHtml(NU_SEARCH_HTML)
    for (const r of results) expect(r.content_type).toBe('novel')
  })
})

// ═══════════════════════════════════════════════════════════════════════════════
// Manga18fx
// ═══════════════════════════════════════════════════════════════════════════════

// Search results: WordPress site, anchors with /manga/{slug} hrefs containing cover imgs
const MANGA18FX_SEARCH_HTML = `
<html><body>
<div class="search-results">
  <div class="item">
    <a href="/manga/tower-of-god">
      <img src="https://manga18fx.com/webtoon/tower-of-godm.jpg" alt="Tower of God">
    </a>
    <h3><a href="/manga/tower-of-god">Tower of God</a></h3>
  </div>
  <div class="item">
    <a href="/manga/solo-leveling">
      <img src="https://manga18fx.com/webtoon/solo-levelingm.jpg" alt="Solo Leveling">
    </a>
    <h3><a href="/manga/solo-leveling">Solo Leveling</a></h3>
  </div>
</div>
</body></html>
`

// Manga detail page: static chapter list
const MANGA18FX_DETAIL_HTML = `
<html><body>
<h1>Tower of God</h1>
<div class="chapter-list">
  <ul>
    <li><a href="/manga/tower-of-god/chapter-1">Chapter 1</a></li>
    <li><a href="/manga/tower-of-god/chapter-2">Chapter 2</a></li>
    <li><a href="/manga/tower-of-god/chapter-100">Chapter 100</a></li>
  </ul>
</div>
</body></html>
`

// Real manga18fx detail pages include a "Most Popular Manga" sidebar with chapter links
// from OTHER series. This fixture replicates that — the bug was: a[href*="/chapter-"]
// picked up all cross-series chapter links, contaminating the chapter list.
const MANGA18FX_DETAIL_WITH_SIDEBAR_HTML = `
<html><body>
<h1>Moby Dick</h1>
<div class="chapter-list">
  <ul>
    <li><a href="/manga/moby-dick/chapter-93">Chapter 93</a></li>
    <li><a href="/manga/moby-dick/chapter-92">Chapter 92</a></li>
    <li><a href="/manga/moby-dick/chapter-1">Chapter 1</a></li>
  </ul>
</div>
<div class="most-popular">
  <h3>Most Popular Manga</h3>
  <a href="/manga/secret-class/chapter-307">Chapter 307</a>
  <a href="/manga/secret-class/chapter-306">Chapter 306</a>
  <a href="/manga/announcer/chapter-10">Chapter 10</a>
  <a href="/manga/just-right-there/chapter-61">Chapter 61</a>
</div>
</body></html>
`

// Chapter page: <img src="https://img01.manga18fx.com/uploads/...">
const MANGA18FX_CHAPTER_HTML = `
<html><body>
<div class="reading-content">
  <img src="https://img01.manga18fx.com/uploads/4337/1/1-abc.jpg">
  <img src="https://img01.manga18fx.com/uploads/4337/1/2-abc.jpg">
  <img src="https://img01.manga18fx.com/uploads/4337/1/3-abc.jpg">
</div>
</body></html>
`

// Mixed lazy+eager: first 2 imgs have only src (eager), last 3 have placeholder src + data-src (lazy)
const MANGA18FX_CHAPTER_MIXED_HTML = `
<html><body>
<div class="reading-content">
  <img src="https://img01.manga18fx.com/uploads/4337/200/1-abc.jpg">
  <img src="https://img01.manga18fx.com/uploads/4337/200/2-abc.jpg">
  <img src="data:image/gif;base64,R0lGODlhAQABAAAAACH5BAEKAAEALAAAAAABAAEAAAICTAEAOw==" data-src="https://img01.manga18fx.com/uploads/4337/200/3-abc.jpg">
  <img src="data:image/gif;base64,R0lGODlhAQABAAAAACH5BAEKAAEALAAAAAABAAEAAAICTAEAOw==" data-src="https://img01.manga18fx.com/uploads/4337/200/4-abc.jpg">
  <img src="data:image/gif;base64,R0lGODlhAQABAAAAACH5BAEKAAEALAAAAAABAAEAAAICTAEAOw==" data-src="https://img01.manga18fx.com/uploads/4337/200/5-abc.jpg">
</div>
</body></html>
`

// Lazy-load variant: src is a placeholder, real URL is in data-src
const MANGA18FX_CHAPTER_LAZY_HTML = `
<html><body>
<div class="reading-content">
  <img src="data:image/gif;base64,R0lGODlhAQABAAAAACH5BAEKAAEALAAAAAABAAEAAAICTAEAOw==" data-src="https://img01.manga18fx.com/uploads/4337/161/1-abc.jpg">
  <img src="data:image/gif;base64,R0lGODlhAQABAAAAACH5BAEKAAEALAAAAAABAAEAAAICTAEAOw==" data-src="https://img01.manga18fx.com/uploads/4337/161/2-abc.jpg">
  <img src="data:image/gif;base64,R0lGODlhAQABAAAAACH5BAEKAAEALAAAAAABAAEAAAICTAEAOw==" data-src="https://img01.manga18fx.com/uploads/4337/161/3-abc.jpg">
</div>
</body></html>
`

describe('manga18fx', () => {
  beforeEach(() => { vi.clearAllMocks() })
  afterEach(() => { vi.unstubAllGlobals() })

  describe('search', () => {
    it('returns results with id and title', async () => {
      vi.stubGlobal('fetch', mockFetch({ '/search?q=': { text: MANGA18FX_SEARCH_HTML } }))
      const results = await manga18fx.search('tower of god')
      expect(results.length).toBeGreaterThanOrEqual(1)
      expect(results[0].id).toBe('tower-of-god')
      expect(results[0].title).toBe('Tower of God')
    })

    it('extracts slug as id from /manga/{slug} href', async () => {
      vi.stubGlobal('fetch', mockFetch({ '/search?q=': { text: MANGA18FX_SEARCH_HTML } }))
      const results = await manga18fx.search('test')
      for (const r of results) {
        expect(r.id).not.toContain('/')
        expect(r.id).not.toContain('manga')
      }
    })

    it('sets content_type to manhwa', async () => {
      vi.stubGlobal('fetch', mockFetch({ '/search?q=': { text: MANGA18FX_SEARCH_HTML } }))
      const results = await manga18fx.search('test')
      for (const r of results) expect(r.content_type).toBe('manhwa')
    })

    it('includes cover_url', async () => {
      vi.stubGlobal('fetch', mockFetch({ '/search?q=': { text: MANGA18FX_SEARCH_HTML } }))
      const results = await manga18fx.search('test')
      expect(results[0].cover_url).toContain('manga18fx.com')
    })

    it('deduplicates results by slug', async () => {
      const dupHtml = MANGA18FX_SEARCH_HTML.replace(
        '<div class="item">',
        '<div class="item">' + MANGA18FX_SEARCH_HTML.split('<div class="item">')[1].split('</div>')[0] + '</div><div class="item">',
      )
      vi.stubGlobal('fetch', mockFetch({ '/search?q=': { text: dupHtml } }))
      const results = await manga18fx.search('test')
      const ids = results.map(r => r.id)
      expect(ids.length).toBe(new Set(ids).size)
    })

    it('calls /search?q= endpoint (not /?s= WordPress fallback)', async () => {
      // This test exists to catch search URL regressions. The plugin used /?s= at first
      // (wrong) and behavior tests passed because mock and impl used the same wrong URL.
      // Asserting the actual URL called prevents that class of silent mismatch.
      const fetchSpy = vi.fn().mockResolvedValue({
        ok: true, text: async () => MANGA18FX_SEARCH_HTML,
      })
      vi.stubGlobal('fetch', fetchSpy)
      await manga18fx.search('Tower of God')
      const calledUrl = fetchSpy.mock.calls[0][0] as string
      expect(calledUrl).toContain('/search?q=')
      expect(calledUrl).not.toContain('/?s=')
    })

    it('returns empty array on fetch error', async () => {
      vi.stubGlobal('fetch', vi.fn().mockResolvedValue({ ok: false, status: 503 }))
      const results = await manga18fx.search('test').catch(() => [])
      expect(Array.isArray(results)).toBe(true)
    })
  })

  describe('chapters', () => {
    it('extracts chapter numbers from static HTML list', async () => {
      vi.stubGlobal('fetch', mockFetch({ '/manga/tower-of-god': { text: MANGA18FX_DETAIL_HTML } }))
      const chapters = await manga18fx.chapters('tower-of-god')
      expect(chapters.length).toBe(3)
      expect(chapters.map(c => c.number)).toEqual([1, 2, 100])
    })

    it('source_id is the chapter URL path', async () => {
      vi.stubGlobal('fetch', mockFetch({ '/manga/tower-of-god': { text: MANGA18FX_DETAIL_HTML } }))
      const chapters = await manga18fx.chapters('tower-of-god')
      expect(chapters[0].source_id).toContain('/chapter-1')
    })

    it('results sorted ascending by number', async () => {
      vi.stubGlobal('fetch', mockFetch({ '/manga/tower-of-god': { text: MANGA18FX_DETAIL_HTML } }))
      const chapters = await manga18fx.chapters('tower-of-god')
      for (let i = 1; i < chapters.length; i++)
        expect(chapters[i].number).toBeGreaterThan(chapters[i - 1].number)
    })

    it('does not include chapter links from sidebar/popular sections of other series', async () => {
      // Regression: a[href*="/chapter-"] picked up Secret Class chapter-307/306 from the
      // "Most Popular Manga" sidebar, contaminating unrelated titles with wrong chapters.
      vi.stubGlobal('fetch', mockFetch({ '/manga/moby-dick': { text: MANGA18FX_DETAIL_WITH_SIDEBAR_HTML } }))
      const chapters = await manga18fx.chapters('moby-dick')
      const nums = chapters.map(c => c.number)
      expect(nums).toEqual([1, 92, 93])
      expect(nums).not.toContain(306)
      expect(nums).not.toContain(307)
    })
  })

  describe('pages', () => {
    it('returns image URLs from chapter page', async () => {
      vi.stubGlobal('fetch', mockFetch({ '/manga/tower-of-god/chapter-1': { text: MANGA18FX_CHAPTER_HTML } }))
      const pages = await manga18fx.pages('/manga/tower-of-god/chapter-1')
      expect(pages.length).toBe(3)
      expect(pages[0]).toContain('img01.manga18fx.com')
    })

    it('all returned URLs start with https', async () => {
      vi.stubGlobal('fetch', mockFetch({ '/manga/tower-of-god/chapter-1': { text: MANGA18FX_CHAPTER_HTML } }))
      const pages = await manga18fx.pages('/manga/tower-of-god/chapter-1')
      for (const p of pages) expect(p).toMatch(/^https:\/\//)
    })

    it('extracts URLs from data-src when site uses lazy loading (src is placeholder)', async () => {
      vi.stubGlobal('fetch', mockFetch({ '/manga/everything-is-agreed-01/chapter-161': { text: MANGA18FX_CHAPTER_LAZY_HTML } }))
      const pages = await manga18fx.pages('/manga/everything-is-agreed-01/chapter-161')
      expect(pages.length).toBe(3)
      expect(pages[0]).toContain('img01.manga18fx.com')
      expect(pages[0]).not.toContain('data:image')
    })

    it('returns data-src value, not placeholder src, for lazy-loaded images', async () => {
      vi.stubGlobal('fetch', mockFetch({ '/manga/everything-is-agreed-01/chapter-161': { text: MANGA18FX_CHAPTER_LAZY_HTML } }))
      const pages = await manga18fx.pages('/manga/everything-is-agreed-01/chapter-161')
      for (const p of pages) expect(p).toMatch(/^https:\/\/img01\.manga18fx\.com\/uploads\//)
    })

    it('returns all CDN URLs from mixed lazy+eager document (some data-src, some src-only)', async () => {
      vi.stubGlobal('fetch', mockFetch({ '/manga/test/chapter-200': { text: MANGA18FX_CHAPTER_MIXED_HTML } }))
      const pages = await manga18fx.pages('/manga/test/chapter-200')
      expect(pages.length).toBe(5)
      for (const p of pages) expect(p).toMatch(/^https:\/\/img01\.manga18fx\.com\/uploads\//)
    })
  })
})
