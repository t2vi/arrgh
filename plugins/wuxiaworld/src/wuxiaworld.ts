import * as cheerio from 'cheerio'
import TurndownService from 'turndown'

const BASE = 'https://www.wuxiaworld.com'
const UA = 'Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/124.0.0.0 Safari/537.36'
const td = new TurndownService({ headingStyle: 'atx', bulletListMarker: '-' })

async function getJson<T>(url: string): Promise<T> {
  const res = await fetch(url, { headers: { 'User-Agent': UA, 'Accept': 'application/json' } })
  if (!res.ok) throw new Error(`wuxiaworld: ${url} returned ${res.status}`)
  return res.json() as Promise<T>
}

async function getHtml(url: string): Promise<string> {
  const res = await fetch(url, { headers: { 'User-Agent': UA } })
  if (!res.ok) throw new Error(`wuxiaworld: ${url} returned ${res.status}`)
  return res.text()
}

// ── Types ─────────────────────────────────────────────────────────────────────

interface WuxiaSearchItem {
  id: number
  slug: string
  name: string
  coverUrl?: string
  status?: number
  authorName?: string | null
  synopsis?: string | null
  tags?: string[]
  genres?: string[]
}

interface WuxiaSearchResponse {
  items: WuxiaSearchItem[]
}

interface EmbeddedChapter {
  slug: string
  name?: string
  offset?: number
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

function mapStatus(s?: number): string {
  if (s === 0) return 'completed'
  if (s === 1) return 'ongoing'
  return 'unknown'
}

// ── Search ────────────────────────────────────────────────────────────────────

export async function search(query: string): Promise<SearchItem[]> {
  const q = encodeURIComponent(query)
  const data = await getJson<WuxiaSearchResponse>(`${BASE}/api/novels/search?query=${q}&pageSize=20`)
  return (data.items ?? []).map((s) => ({
    id: s.slug,
    title: s.name,
    description: s.synopsis ? s.synopsis.replace(/<[^>]+>/g, '').trim() || null : null,
    cover_url: s.coverUrl ?? null,
    status: mapStatus(s.status),
    author: s.authorName ?? null,
    year: null,
    tags: [...(s.tags ?? []), ...(s.genres ?? [])].join(', ') || null,
    content_type: 'novel',
  }))
}

// ── Chapters ──────────────────────────────────────────────────────────────────
// The wuxiaworld SPA lazy-loads chapter lists — no public bulk REST API.
// We extract firstChapter from the embedded JSON state in the chapters page.
// source_id format: "novelSlug/chapterSlug" → used to build the chapter URL.

export async function chapters(novelSlug: string): Promise<ChapterItem[]> {
  const html = await getHtml(`${BASE}/novel/${novelSlug}/chapters`)
  const firstChapter = extractEmbeddedChapter(html, 'firstChapter')
  if (!firstChapter) return []

  return [{
    source_id: `${novelSlug}/${firstChapter.slug}`,
    number: firstChapter.offset ?? 1,
    volume: null,
    title: firstChapter.name ?? null,
  }]
}

function extractEmbeddedChapter(html: string, key: string): EmbeddedChapter | null {
  const marker = `"${key}":{`
  const idx = html.indexOf(marker)
  if (idx < 0) return null
  const start = idx + marker.length - 1
  let depth = 0; let end = start
  for (let i = start; i < html.length; i++) {
    if (html[i] === '{') depth++
    else if (html[i] === '}') { depth--; if (depth === 0) { end = i + 1; break } }
  }
  try {
    return JSON.parse(html.slice(start, end)) as EmbeddedChapter
  } catch {
    return null
  }
}

// ── Chapter text ──────────────────────────────────────────────────────────────
// source_id = "novelSlug/chapterSlug" → URL: /novel/novelSlug/chapterSlug

export async function chapterText(chapterId: string): Promise<string> {
  const url = `${BASE}/novel/${chapterId}`
  const html = await getHtml(url)
  const $ = cheerio.load(html)
  const content = $('.chapter-content').first().html() || ''
  return td.turndown(content).trim()
}
