import * as cheerio from 'cheerio'
import type { AnyNode } from 'domhandler'

const BASE = 'https://toonily.com'

// ── Minimal browser interface (duck-typed, no playwright dep in bundle) ───────

interface BrowserPage {
  goto(url: string, opts?: { waitUntil?: string; timeout?: number }): Promise<unknown>
  content(): Promise<string>
  close(): Promise<void>
}
interface BrowserContextLike {
  addCookies(cookies: Array<{ name: string; value: string; domain: string; path?: string }>): Promise<void>
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

// ── HTTP helper ───────────────────────────────────────────────────────────────

async function getPage(url: string): Promise<string> {
  const browser = await _ctx!.getBrowser()
  const bctx = await browser.newContext()
  await bctx.addCookies([{ name: 'toonily-mature', value: '1', domain: 'toonily.com', path: '/' }])
  const page = await bctx.newPage()
  try {
    await page.goto(url, { waitUntil: 'domcontentloaded', timeout: 60_000 })
    return await page.content()
  } finally {
    await bctx.close()
  }
}

// ── HTML helpers ──────────────────────────────────────────────────────────────

function imgSrc($el: cheerio.Cheerio<AnyNode>): string | null {
  return $el.attr('src') ?? $el.attr('data-lazy-src') ?? $el.attr('data-src') ?? null
}

function slugFromUrl(url: string): string {
  return url.replace(/\/$/, '').split('/').pop() ?? ''
}

// ── Output types (Source Plugin Protocol) ────────────────────────────────────

export interface SearchItem {
  id: string
  title: string
  description: string | null
  cover_url: string | null
  status: string
  author: string | null
  year: number | null
  tags: string | null
}

export interface ChapterItem {
  source_id: string
  number: number
  volume: number | null
  title: string | null
}

export interface MetaResponse {
  description: string | null
  cover_url: string | null
  chapter_count: number
  tags: string | null
}

// ── Scrapers ──────────────────────────────────────────────────────────────────

export async function search(query: string): Promise<SearchItem[]> {
  const slug = query.toLowerCase().replace(/[^a-z0-9]+/g, '-').replace(/^-|-$/g, '')
  const html = await getPage(`${BASE}/search/${slug}?author&artist&adult`)
  const $ = cheerio.load(html)
  const results: SearchItem[] = []

  $('.page-item-detail.manga').each((_, el) => {
    const $el = $(el)
    const $link = $el.find('.post-title a').first()
    const href = $link.attr('href') ?? ''
    const slug = slugFromUrl(href)
    if (!slug) return

    results.push({
      id: slug,
      title: $link.text().trim(),
      description: null,
      cover_url: imgSrc($el.find('.c-image-hover img').first()),
      status: 'ongoing',
      author: null,
      year: null,
      tags: null,
    })
  })

  return results
}

export async function trending(): Promise<SearchItem[]> {
  const html = await getPage(`${BASE}/manga/?m_orderby=trending`)
  const $ = cheerio.load(html)
  const results: SearchItem[] = []

  $('.page-item-detail.manga').each((_, el) => {
    const $el = $(el)
    const $link = $el.find('.post-title a').first()
    const href = $link.attr('href') ?? ''
    const slug = slugFromUrl(href)
    if (!slug) return

    results.push({
      id: slug,
      title: $link.text().trim(),
      description: null,
      cover_url: imgSrc($el.find('.c-image-hover img').first()),
      status: 'ongoing',
      author: null,
      year: null,
      tags: null,
    })
  })

  return results
}

export async function meta(slug: string): Promise<MetaResponse> {
  const html = await getPage(`${BASE}/serie/${slug}/`)
  const $ = cheerio.load(html)

  const description = $('.summary__content p').first().text().trim() || null
  const cover_url = imgSrc($('.summary_image img').first())
  const chapter_count = $('.wp-manga-chapter').length

  const EXPLICIT_GENRES = new Set(['adult', 'mature', 'hentai', 'smut', '18+'])
  const genres = $('.genres-content a')
    .map((_, a) => {
      const raw = $(a).text().trim()
      return EXPLICIT_GENRES.has(raw.toLowerCase()) ? 'adult' : raw
    })
    .get()
    .filter(Boolean)

  return {
    description,
    cover_url: cover_url ?? null,
    chapter_count,
    tags: genres.length ? genres.join(', ') : null,
  }
}

export async function chapters(slug: string): Promise<ChapterItem[]> {
  const html = await getPage(`${BASE}/serie/${slug}/`)
  const $ = cheerio.load(html)
  const items: ChapterItem[] = []

  $('.wp-manga-chapter').each((_, el) => {
    const href = $(el).find('a').attr('href') ?? ''
    const parts = href.replace(/\/$/, '').split('/')
    const chapterSlug = parts.pop() ?? ''
    const mangaSlug = parts.pop() ?? ''
    if (!chapterSlug || !mangaSlug) return

    const match = chapterSlug.match(/chapter-(\d+)(?:-(\d+))?/)
    if (!match) return
    const major = parseInt(match[1]!, 10)
    const minor = match[2] ? parseInt(match[2], 10) : 0
    const number = minor > 0 ? parseFloat(`${major}.${minor}`) : major

    items.push({ source_id: `${mangaSlug}/${chapterSlug}`, number, volume: null, title: null })
  })

  return items.reverse()
}

export async function pages(chapterSourceId: string): Promise<{ url: string; referer: string }[]> {
  const chapterUrl = `${BASE}/serie/${chapterSourceId}/`
  const html = await getPage(chapterUrl)
  const $ = cheerio.load(html)
  const results: { url: string; referer: string }[] = []

  $('.page-break img').each((_, el) => {
    const src = imgSrc($(el))
    if (src) results.push({ url: src.trim(), referer: chapterUrl })
  })

  return results
}
