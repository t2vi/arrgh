import 'dotenv/config'
import * as cheerio from 'cheerio'
import type { AnyNode } from 'domhandler'

const BASE = 'https://toonily.com'
const FLARESOLVERR = () => process.env.FLARESOLVERR_URL ?? 'http://flaresolverr:8191'

// ── FlareSolverr client ───────────────────────────────────────────────────────

interface FlareSolverrResult {
  status: string
  solution: { response: string; status: number; url: string }
}

const MATURE_COOKIE = [{ name: 'toonily-mature', value: '1' }]

async function flareFetch(cmd: Record<string, unknown>): Promise<string> {
  const res = await fetch(`${FLARESOLVERR()}/v1`, {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify({ maxTimeout: 60000, cookies: MATURE_COOKIE, ...cmd }),
  })
  if (!res.ok) throw new Error(`FlareSolverr HTTP ${res.status}`)
  const data = (await res.json()) as FlareSolverrResult
  if (data.status !== 'ok') throw new Error(`FlareSolverr error: ${data.status}`)
  if (data.solution.status !== 200)
    throw new Error(`Toonily ${data.solution.status}`)
  return data.solution.response
}

async function getPage(url: string): Promise<string> {
  return flareFetch({ cmd: 'request.get', url })
}

// ── HTML helpers ──────────────────────────────────────────────────────────────

function imgSrc($el: cheerio.Cheerio<AnyNode>): string | null {
  return $el.attr('src') ?? $el.attr('data-lazy-src') ?? $el.attr('data-src') ?? null
}

function slugFromUrl(url: string): string {
  // https://toonily.com/serie/tower-of-god-acc4fd16/ → tower-of-god-acc4fd16
  return url.replace(/\/$/, '').split('/').pop() ?? ''
}

function normalizeStatus(raw: string): string {
  const s = raw.trim().toLowerCase()
  if (s.includes('complet')) return 'completed'
  if (s.includes('hiatus') || s.includes('paused')) return 'hiatus'
  return 'ongoing'
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
  const html = await getPage(`${BASE}/?s=${encodeURIComponent(query)}&post_type=wp-manga`)
  const $ = cheerio.load(html)
  const results: SearchItem[] = []

  $('.page-item-detail.manga').each((_, el) => {
    const $el = $(el)
    const $link = $el.find('.post-title a').first()
    const href = $link.attr('href') ?? ''
    const slug = slugFromUrl(href)
    if (!slug) return

    const title = $link.text().trim()
    const cover = imgSrc($el.find('.c-image-hover img').first())

    results.push({
      id: slug,
      title,
      description: null,
      cover_url: cover,
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

    const title = $link.text().trim()
    const cover = imgSrc($el.find('.c-image-hover img').first())

    results.push({
      id: slug,
      title,
      description: null,
      cover_url: cover,
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
  const tags = genres.length ? genres.join(', ') : null

  return { description, cover_url: cover_url ?? null, chapter_count, tags }
}

export async function chapters(slug: string): Promise<ChapterItem[]> {
  const html = await getPage(`${BASE}/serie/${slug}/`)
  const $ = cheerio.load(html)
  const items: ChapterItem[] = []

  $('.wp-manga-chapter').each((_, el) => {
    const href = $(el).find('a').attr('href') ?? ''
    // href: https://toonily.com/serie/{manga_slug}/{chapter_slug}/
    const parts = href.replace(/\/$/, '').split('/')
    const chapterSlug = parts.pop() ?? ''
    const mangaSlug = parts.pop() ?? ''
    if (!chapterSlug || !mangaSlug) return

    const source_id = `${mangaSlug}/${chapterSlug}`

    // chapter-242 → 242 | chapter-242-5 → 242.5
    const match = chapterSlug.match(/chapter-(\d+)(?:-(\d+))?/)
    if (!match) return
    const major = parseInt(match[1]!, 10)
    const minor = match[2] ? parseInt(match[2], 10) : 0
    const number = minor > 0 ? parseFloat(`${major}.${minor}`) : major

    items.push({ source_id, number, volume: null, title: null })
  })

  // Page renders newest-first; reverse for ascending
  return items.reverse()
}

export async function pages(chapterSourceId: string): Promise<{ url: string; referer: string }[]> {
  // chapterSourceId = "{manga_slug}/{chapter_slug}"
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
