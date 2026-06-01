import * as cheerio from 'cheerio'

const BASE = 'https://manga18fx.com'
const UA = 'Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/124.0.0.0 Safari/537.36'
// ── CloakBrowser context (injected by plugin-host) ───────────────────────────

interface BrowserPage {
  goto(url: string, opts?: { waitUntil?: string; timeout?: number }): Promise<unknown>
  content(): Promise<string>
  close(): Promise<void>
}
interface BrowserContextLike {
  newPage(): Promise<BrowserPage>
  close(): Promise<void>
}
interface BrowserLike {
  newContext(opts?: Record<string, unknown>): Promise<BrowserContextLike>
  isConnected(): boolean
}
export interface PluginContext {
  getBrowser: () => Promise<BrowserLike>
  logger: typeof console
}

let _ctx: PluginContext | null = null
export function setContext(ctx: PluginContext): void { _ctx = ctx }

async function getPage(url: string): Promise<string> {
  if (_ctx) {
    const browser = await _ctx.getBrowser()
    const bctx = await browser.newContext()
    const page = await bctx.newPage()
    try {
      await page.goto(url, { waitUntil: 'domcontentloaded', timeout: 60_000 })
      return await page.content()
    } finally {
      await bctx.close()
    }
  }
  // Fallback: direct fetch (tests / no-CF environments)
  const res = await fetch(url, { headers: { 'User-Agent': UA } })
  if (!res.ok) throw new Error(`${url} returned ${res.status}`)
  return res.text()
}



async function getHtml(url: string): Promise<string> {
  const res = await fetch(url, { headers: { 'User-Agent': UA } })
  if (!res.ok) throw new Error(`manga18fx: ${url} returned ${res.status}`)
  return res.text()
}

export interface SearchItem {
  id: string
  title: string
  description: string | null
  cover_url: string | null
  status: string
  author: string | null
  year: number | null
  tags: string | null
  content_type: string
}

export interface ChapterItem {
  source_id: string
  number: number
  volume: number | null
  title: string | null
}

// ── Search ────────────────────────────────────────────────────────────────────

export async function search(query: string): Promise<SearchItem[]> {
  const q = query.replace(/-/g, ' ')
  const html = await getPage(`${BASE}/search?q=${encodeURIComponent(q)}`)
  const $ = cheerio.load(html)
  const results: SearchItem[] = []
  const seen = new Set<string>()

  // Find anchors pointing at manga root pages that contain cover thumbnails.
  // Pattern: <a href="/manga/{slug}"><img src="...webtoon/{slug}m.jpg" alt="Title"></a>
  $('a[href^="/manga/"]').each((_, el) => {
    const $a = $(el)
    const href = $a.attr('href') ?? ''
    // Must be a manga root page — no sub-path like /chapter-n
    if (!/^\/manga\/[^/]+\/?$/.test(href)) return

    const $img = $a.find('img').first()
    if (!$img.length) return

    const slug = href.replace(/^\/manga\//, '').replace(/\/$/, '')
    if (!slug || seen.has(slug)) return
    seen.add(slug)

    const title = $img.attr('alt')?.trim()
      || $a.parent().find('h3, h4, .title, .name').first().text().trim()
      || ''
    if (!title) return

    results.push({
      id: slug,
      title,
      cover_url: $img.attr('src') ?? `${BASE}/webtoon/${slug}m.jpg`,
      status: 'ongoing',
      content_type: 'manhwa',
      description: null,
      author: null,
      year: null,
      tags: null,
    })
  })

  return results
}

// ── Chapters ──────────────────────────────────────────────────────────────────
// Manga18fx renders chapter lists as static HTML on the manga detail page.

export async function chapters(seriesId: string): Promise<ChapterItem[]> {
  const html = await getPage(`${BASE}/manga/${seriesId}`)
  const $ = cheerio.load(html)
  const seen = new Map<number, ChapterItem>()

  $(`a[href^="/manga/${seriesId}/chapter-"]`).each((_, el) => {
    const href = $(el).attr('href') ?? ''
    const m = href.match(/\/chapter-(\d+(?:\.\d+)?)/)
    if (!m) return
    const num = parseFloat(m[1])
    if (isNaN(num) || seen.has(num)) return
    seen.set(num, { source_id: href, number: num, volume: null, title: null })
  })

  return Array.from(seen.values()).sort((a, b) => a.number - b.number)
}

// ── Pages ─────────────────────────────────────────────────────────────────────
// Chapter pages serve images from imgXX.manga18fx.com CDN subdomains

export async function pages(chapterId: string): Promise<string[]> {
  const url = chapterId.startsWith('http')
    ? chapterId
    : `${BASE}${chapterId.startsWith('/') ? '' : '/'}${chapterId}`
  const html = await getPage(url)
  const $ = cheerio.load(html)
  const urls: string[] = []

  // Lazy-load: data-src holds real URL, src may be placeholder; match any manga18fx CDN subdomain
  $('img[src*=".manga18fx.com"], img[data-src*=".manga18fx.com"]').each((_, el) => {
    const src = $(el).attr('data-src') || $(el).attr('src')
    if (src?.startsWith('http')) urls.push(src)
  })

  return urls
}
