// Behavior tests for ADR 0031 new plugins.
// Each suite stubs fetch with minimal HTML/JSON fixtures and asserts on output shapes.
// Tests are RED until plugin dirs are created; GREEN after implementation.
// Fixtures define the parsing contract — implementations must match these structures.

import { describe, it, expect, beforeEach, afterEach } from 'vitest'

// ── Imports (fail until plugin dirs exist) ────────────────────────────────────

import * as asurascans   from '../../plugins/asurascans/src/index'
import * as wuxiaworld   from '../../plugins/wuxiaworld/src/index'
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
    <a href="/comics/solo-leveling-abc123" class="slide-link block">
      <img src="https://gg.asuracomic.net/storage/covers/solo-leveling.jpg" class="rounded" alt="Solo Leveling">
      <div class="col-span-8 flex flex-col">
        <span class="block font-bold text-white">Solo Leveling</span>
        <span class="text-xs">Manhwa</span>
        <span class="text-xs text-[#ff7e2e]">ONGOING</span>
      </div>
    </a>
  </div>
  <div class="group/tipmanga">
    <a href="/comics/return-of-the-mount-hua-sect-xyz789" class="slide-link block">
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
    vi.stubGlobal('fetch', mockFetch({ 'asurascans': { text: ASURA_SEARCH_HTML } }))
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
    vi.stubGlobal('fetch', mockFetch({ 'asurascans': { text: ASURA_SEARCH_HTML } }))
    const [first] = await asurascans.search('solo leveling')
    expect(first.id).toBe('solo-leveling-abc123')
    expect(first.title).toBe('Solo Leveling')
    expect(first.content_type).toBe('manhwa')
  })

  it('status is lowercase normalised', async () => {
    vi.stubGlobal('fetch', mockFetch({ 'asurascans': { text: ASURA_SEARCH_HTML } }))
    const [first] = await asurascans.search('solo leveling')
    expect(first.status).toBe('ongoing')
  })

  it('returns empty array when no results', async () => {
    vi.stubGlobal('fetch', mockFetch({ 'asurascans': { text: '<div></div>' } }))
    const results = await asurascans.search('nothing')
    expect(results).toEqual([])
  })

  it('throws on non-ok response', async () => {
    vi.stubGlobal('fetch', mockFetch({ 'asurascans': { ok: false } }))
    await expect(asurascans.search('test')).rejects.toThrow()
  })
})

describe('asurascans — chapters', () => {
  it('returns chapters with source_id and number', async () => {
    vi.stubGlobal('fetch', mockFetch({ 'asurascans': { text: ASURA_CHAPTERS_HTML } }))
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
    vi.stubGlobal('fetch', mockFetch({ 'asurascans': { text: ASURA_CHAPTERS_HTML } }))
    const chapters = await asurascans.chapters('solo-leveling-abc123')
    const ch180 = chapters.find((c) => c.number === 180)
    expect(ch180).toBeDefined()
    expect(ch180!.source_id).toContain('solo-leveling-abc123')
  })
})

describe('asurascans — pages', () => {
  it('returns image URL array', async () => {
    vi.stubGlobal('fetch', mockFetch({ 'asurascans': { text: ASURA_PAGES_HTML } }))
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
// WuxiaWorld
// ═══════════════════════════════════════════════════════════════════════════════

const WUXIA_SEARCH_JSON = {
  items: [
    {
      id: 12,
      slug: 'swallowed-star',
      name: 'Swallowed Star',
      coverUrl: 'https://cdn.wuxiaworld.com/covers/swallowed-star.jpg',
      status: 0,
      authorName: 'I Eat Tomatoes',
      tags: ['Chinese', 'Completed'],
      genres: ['Sci-fi', 'Action'],
    },
    {
      id: 34,
      slug: 'martial-world',
      name: 'Martial World',
      coverUrl: 'https://cdn.wuxiaworld.com/covers/martial-world.jpg',
      status: 0,
      authorName: 'Cocooned Cow',
      tags: ['Chinese', 'Completed'],
      genres: ['Martial Arts'],
    },
  ],
}

// chapters() parses embedded React Query state from the chapters page HTML.
// Both groups use fromChapterNumber.units=1 — matching WuxiaWorld's real decimal format
// where all groups report units=1 (chapters are sub-1.0 decimals internally).
// The implementation must use cumulative numbering, not fromChapterNumber.units + i.
const WUXIA_CHAPTERS_HTML = `<html><body><script>
window.__REACT_QUERY_STATE__ = {"queries":[{"queryKey":["novel","swallowed-star",null],"state":{"data":{"item":{
  "chapterInfo":{
    "chapterCount":{"value":3},
    "firstChapter":{"slug":"swallowed-star-chapter-1","name":"Chapter 1 — The Swift as Lightning Technique","offset":1},
    "chapterGroups":[
      {"id":1,"title":"Volume 1","order":1,
       "fromChapterNumber":{"units":1,"nanos":0},"toChapterNumber":{"units":1,"nanos":999999900},
       "counts":{"total":2,"advance":0,"normal":2},"chapterList":[]},
      {"id":2,"title":"Volume 2","order":2,
       "fromChapterNumber":{"units":1,"nanos":0},"toChapterNumber":{"units":1,"nanos":999999900},
       "counts":{"total":1,"advance":0,"normal":1},"chapterList":[]}
    ]
  }
}}}}]};
</script></body></html>`

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
  it('returns all chapters from chapterGroups count', async () => {
    vi.stubGlobal('fetch', mockFetch({ 'wuxiaworld': { text: WUXIA_CHAPTERS_HTML } }))
    const chapters = await wuxiaworld.chapters('swallowed-star')
    expect(Array.isArray(chapters)).toBe(true)
    // fixture has chapterCount=3 across 2 groups (2+1)
    expect(chapters.length).toBe(3)
    for (const ch of chapters) {
      expect(ch).toHaveProperty('source_id')
      expect(ch).toHaveProperty('number')
      expect(typeof ch.number).toBe('number')
    }
  })

  it('chapter 1 source_id uses real slug from firstChapter', async () => {
    vi.stubGlobal('fetch', mockFetch({ 'wuxiaworld': { text: WUXIA_CHAPTERS_HTML } }))
    const [ch1] = await wuxiaworld.chapters('swallowed-star')
    expect(ch1.source_id).toBe('swallowed-star/swallowed-star-chapter-1')
    expect(ch1.number).toBe(1)
    expect(ch1.title).toBe('Chapter 1 — The Swift as Lightning Technique')
  })

  it('chapters 2+ use numeric source_id {novelSlug}/chapter/{N}', async () => {
    vi.stubGlobal('fetch', mockFetch({ 'wuxiaworld': { text: WUXIA_CHAPTERS_HTML } }))
    const chapters = await wuxiaworld.chapters('swallowed-star')
    expect(chapters[1].source_id).toBe('swallowed-star/chapter/2')
    expect(chapters[2].source_id).toBe('swallowed-star/chapter/3')
  })

  it('volume set to chapterGroup order', async () => {
    vi.stubGlobal('fetch', mockFetch({ 'wuxiaworld': { text: WUXIA_CHAPTERS_HTML } }))
    const chapters = await wuxiaworld.chapters('swallowed-star')
    // group 1 covers chapters 1-2 (order=1), group 2 covers chapter 3 (order=2)
    expect(chapters[0].volume).toBe(1)
    expect(chapters[1].volume).toBe(1)
    expect(chapters[2].volume).toBe(2)
  })

  it('returns empty array when chapterInfo missing', async () => {
    const emptyHtml = `<html><body><script>window.__REACT_QUERY_STATE__ = {"queries":[]};</script></body></html>`
    vi.stubGlobal('fetch', mockFetch({ 'wuxiaworld': { text: emptyHtml } }))
    const chapters = await wuxiaworld.chapters('swallowed-star')
    expect(chapters).toEqual([])
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
