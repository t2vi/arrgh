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
// WuxiaWorld chapter lists are loaded by the SPA via a private API — no public
// bulk REST endpoint exists. We extract chapterInfo from the React Query state
// embedded in the chapters page HTML.
//
// source_id format:
//   chapter 1:   "novelSlug/chapterSlug"   — real slug (SSR-readable)
//   chapters 2+: "novelSlug/chapter/{N}"   — numeric fallback; text may not load
//                                             if WuxiaWorld returns a CSR-only page

interface ChapterGroup {
  id: number
  order: number
  title: string
  fromChapterNumber: { units: number }
  toChapterNumber: { units: number }
  counts: { total: number }
}

interface ChapterInfo {
  chapterCount?: { value: number }
  firstChapter?: EmbeddedChapter
  chapterGroups?: ChapterGroup[]
}

export async function chapters(novelSlug: string): Promise<ChapterItem[]> {
  const html = await getHtml(`${BASE}/novel/${novelSlug}/chapters`)
  const chapterInfo = extractChapterInfo(html)
  if (!chapterInfo || !chapterInfo.chapterGroups?.length) return []

  const { firstChapter, chapterGroups } = chapterInfo
  const result: ChapterItem[] = []

  // WuxiaWorld's fromChapterNumber uses an internal decimal format (units=1 for all groups)
  // not sequential chapter numbers — use cumulative offset to guarantee unique chapter numbers.
  let cumulative = 1
  for (const group of chapterGroups) {
    const count = group.counts.total
    for (let i = 0; i < count; i++) {
      const num = cumulative + i
      const isFirst = num === 1 && firstChapter?.slug
      result.push({
        source_id: isFirst ? `${novelSlug}/${firstChapter!.slug}` : `${novelSlug}/chapter/${num}`,
        number: num,
        volume: group.order,
        title: isFirst ? (firstChapter!.name ?? null) : null,
      })
    }
    cumulative += count
  }

  return result
}

function extractChapterInfo(html: string): ChapterInfo | null {
  const match = html.match(/window\.__REACT_QUERY_STATE__\s*=\s*(\{.*?\});\s*/s)
  if (!match) return null
  try {
    const state = JSON.parse(match[1]) as { queries?: { queryKey?: unknown[]; state?: { data?: { item?: { chapterInfo?: ChapterInfo } } } }[] }
    const novelQuery = state.queries?.find(q => Array.isArray(q.queryKey) && q.queryKey.includes('novel'))
    return novelQuery?.state?.data?.item?.chapterInfo ?? null
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
