import * as cheerio from 'cheerio'

const BASE = 'https://www.novelupdates.com'

// ── Minimal browser interface (duck-typed, no playwright dep in bundle) ───────

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

export function setContext(ctx: PluginContext): void {
  _ctx = ctx
}

async function flareHtml(url: string): Promise<string> {
  const browser = await _ctx!.getBrowser()
  const bctx = await browser.newContext()
  const page = await bctx.newPage()
  try {
    await page.goto(url, { waitUntil: 'domcontentloaded', timeout: 60_000 })
    return await page.content()
  } finally {
    await bctx.close()
  }
}

// ── Types ─────────────────────────────────────────────────────────────────────

export interface SearchResult {
  id: string
  title: string
  cover_url: string | null
  status: string
  content_type: string
}

// ── Search ────────────────────────────────────────────────────────────────────

export async function search(query: string): Promise<SearchResult[]> {
  const url = `${BASE}/?s=${encodeURIComponent(query)}&post_type=series`
  const html = await flareHtml(url)
  return parseSearchHtml(html)
}

// Exported for unit testing
export function parseSearchHtml(html: string): SearchResult[] {
  const $ = cheerio.load(html)
  const results: SearchResult[] = []

  // NovelUpdates search results: each .search_main_box_nu block
  $('.search_main_box_nu').each((_, el) => {
    const $el = $(el)

    // Title + slug: <div class="search_title"><a href="/series/{slug}/">Title</a>
    const $titleLink = $el.find('.search_title a[href*="/series/"]').first()
    const href = $titleLink.attr('href') ?? ''
    const slugMatch = href.match(/\/series\/([^/]+)\//)
    if (!slugMatch) return
    const slug = slugMatch[1]
    const title = $titleLink.text().trim()
    if (!title) return

    // Cover
    const cover = $el.find('.search_img_nu img').first().attr('src') ?? null

    // Status
    const statusRaw = $el.find('.series_latest_status').first().text().trim()
    const status = mapStatus(statusRaw)

    results.push({ id: slug, title, cover_url: cover || null, status, content_type: 'novel' })
  })

  return results
}

function mapStatus(s: string): string {
  switch (s.toLowerCase().trim()) {
    case 'completed': return 'complete'
    case 'ongoing': case 'publishing': return 'ongoing'
    case 'hiatus': return 'hiatus'
    case 'dropped': return 'cancelled'
    default: return 'unknown'
  }
}

// ── Stubs (metadata-only plugin — no chapter downloading) ────────────────────
// ADR 0016: chapters/pages not required for metadata-only plugins but must exist

export async function chapters(_id: string): Promise<never[]> {
  return []
}
