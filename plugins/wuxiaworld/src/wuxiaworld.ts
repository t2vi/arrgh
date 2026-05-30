import * as cheerio from 'cheerio'
import TurndownService from 'turndown'

const BASE = 'https://www.wuxiaworld.com'
const API = 'https://api2.wuxiaworld.com'
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
  id: string
  name: string
  coverUrl?: string
  status?: string
  author?: { name: string }
  genres?: string[]
}

interface WuxiaSearchResponse {
  items: WuxiaSearchItem[]
  total?: number
}

interface WuxiaChapterItem {
  entityId: string
  name?: string
  chapter?: { num: number }
}

interface WuxiaChaptersResponse {
  items: WuxiaChapterItem[]
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
  const q = encodeURIComponent(query)
  const data = await getJson<WuxiaSearchResponse>(`${API}/api/novels/search?query=${q}&pageSize=20`)
  return (data.items ?? []).map((s) => ({
    id: s.id,
    title: s.name,
    description: null,
    cover_url: s.coverUrl ?? null,
    status: (s.status ?? 'unknown').toLowerCase(),
    author: s.author?.name ?? null,
    year: null,
    tags: s.genres?.join(', ') ?? null,
    content_type: 'novel',
  }))
}

// ── Chapters ──────────────────────────────────────────────────────────────────

export async function chapters(novelId: string): Promise<ChapterItem[]> {
  const data = await getJson<WuxiaChaptersResponse>(`${API}/api/novels/${novelId}/chapters?pageSize=2000`)
  const seen = new Map<number, ChapterItem>()

  for (const ch of data.items ?? []) {
    const num = ch.chapter?.num ?? NaN
    if (isNaN(num)) continue
    const item: ChapterItem = {
      source_id: ch.entityId,
      number: num,
      volume: null,
      title: ch.name ?? null,
    }
    if (!seen.has(num)) seen.set(num, item)
  }

  return Array.from(seen.values()).sort((a, b) => a.number - b.number)
}

// ── Chapter text ──────────────────────────────────────────────────────────────

export async function chapterText(chapterId: string): Promise<string> {
  // chapterId = entityId e.g. "swallowed-star/swallowed-star-chapter-1"
  const url = `${BASE}/novel/${chapterId}`
  const html = await getHtml(url)
  const $ = cheerio.load(html)
  const content = $('.chapter-content').html()
    || $('[class*="chapter-content"]').html()
    || $('.reading-content').html()
    || ''
  return td.turndown(content).trim()
}
