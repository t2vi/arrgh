import * as cheerio from 'cheerio'

const BASE = 'https://asurascans.com'
const UA = 'Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/124.0.0.0 Safari/537.36'
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

async function getPage(url: string, waitUntil: string = 'domcontentloaded', delayMs = 0): Promise<string> {
  if (_ctx) {
    const browser = await _ctx.getBrowser()
    const bctx = await browser.newContext()
    const page = await bctx.newPage()
    try {
      await page.goto(url, { waitUntil, timeout: 60_000 })
      if (delayMs > 0) await new Promise(r => setTimeout(r, delayMs))
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
  if (!res.ok) throw new Error(`asurascans: ${url} returned ${res.status}`)
  return res.text()
}

function normalizeStatus(s: string): string {
  return s.toLowerCase().trim()
    .replace('hiatus', 'hiatus')
    .replace('dropped', 'dropped')
}

// ── Types ─────────────────────────────────────────────────────────────────────

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
// AsuraScans search: https://asuracomic.net/browse?q=query

export async function search(query: string): Promise<SearchItem[]> {
  const q = query.replace(/-/g, '')
  const html = await getPage(`${BASE}/browse?q=${encodeURIComponent(q)}`)
  const $ = cheerio.load(html)
  const results: SearchItem[] = []
  const seen = new Set<string>()

  // Asura uses /comics/{slug}-{hash} links
  $('a[href*="/comics/"]').each((_, el) => {
    const $a = $(el)
    const href = $a.attr('href') ?? ''
    // href: /comics/solo-leveling-abc12345
    const m = href.match(/\/comics\/([\w-]+)\/?$/)
    const id = m?.[1]
    if (!id || seen.has(id)) return
    seen.add(id)

    // Title: span.font-bold, or img alt
    const title = $a.find('span.font-bold, .tt').first().text().trim()
      || $a.find('img').first().attr('alt')?.trim()
      || ''
    if (!title) return

    const cover = $a.find('img').first().attr('src') || null
    const statusRaw = $a.find('.status, [class*="status"]').first().text().trim() || 'ongoing'
    // AsuraScans is manhwa-only
    const contentType = $a.find('span.text-xs').eq(0).text().trim().toLowerCase() || 'manhwa'

    results.push({
      id,
      title,
      description: null,
      cover_url: cover,
      status: normalizeStatus(statusRaw),
      author: null,
      year: null,
      tags: null,
      content_type: contentType === 'manhwa' ? 'manhwa' : 'manhwa',
    })
  })

  return results
}

// ── Chapters ──────────────────────────────────────────────────────────────────

export async function chapters(seriesId: string): Promise<ChapterItem[]> {
  // Asura is an Astro/React SPA — chapter links render after JS hydration.
  // Wait 5s after domcontentloaded then extract chapter numbers via regex.
  const html = await getPage(`${BASE}/comics/${seriesId}`, 'domcontentloaded', 5000)
  const seen = new Map<number, ChapterItem>()
  const re = new RegExp(`${seriesId.replace(/[.*+?^${}()|[\]\\]/g, '\\$&')}\/chapter\/(\\d+(?:\\.\\d+)?)`, 'g')
  let m: RegExpExecArray | null
  while ((m = re.exec(html)) !== null) {
    const num = parseFloat(m[1])
    if (!isNaN(num) && !seen.has(num)) {
      seen.set(num, {
        source_id: `/comics/${seriesId}/chapter/${m[1]}`,
        number: num,
        volume: null,
        title: null,
      })
    }
  }
  return Array.from(seen.values()).sort((a, b) => a.number - b.number)
}

// ── Pages ─────────────────────────────────────────────────────────────────────

export async function pages(chapterId: string): Promise<string[]> {
  const url = chapterId.startsWith('http') ? chapterId : `${BASE}${chapterId.startsWith('/') ? '' : '/'}${chapterId}`
  const html = await getPage(url)
  const $ = cheerio.load(html)

  const urls: string[] = []
  // AsuraScans renders images in flex column containers
  $('img[src*="asuracomic"], img[src*="gg.asura"], .object-cover[src]').each((_, el) => {
    const src = $(el).attr('src')
    if (src && src.startsWith('http')) urls.push(src)
  })

  // Fallback: collect all chapter page images
  if (urls.length === 0) {
    $('div.flex img[src^="http"], div[class*="reading"] img[src^="http"]').each((_, el) => {
      const src = $(el).attr('src')
      if (src) urls.push(src)
    })
  }

  return urls
}
