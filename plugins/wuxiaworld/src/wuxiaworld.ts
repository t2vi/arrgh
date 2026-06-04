import TurndownService from 'turndown'

const BASE = 'https://www.wuxiaworld.com'
const UA = 'Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/124.0.0.0 Safari/537.36'
const td = new TurndownService({ headingStyle: 'atx', bulletListMarker: '-' })

// ── Plugin context (kept for interface compatibility) ─────────────────────────
export interface PluginContext {
  getBrowser: () => Promise<unknown>
  logger: typeof console
}
let _ctx: PluginContext | null = null
export function setContext(ctx: PluginContext): void { _ctx = ctx }
void _ctx

// ── Fetch helpers ─────────────────────────────────────────────────────────────

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

// ── Search types ──────────────────────────────────────────────────────────────

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

// ── React Query SSR extraction ────────────────────────────────────────────────
// WuxiaWorld embeds window.__REACT_QUERY_STATE__ = {...}; in page HTML.
// The `;` at end of the assignment (never inside nested JSON) lets the lazy
// regex stop at the right closing brace.

function parseReactQueryState(html: string): Record<string, unknown> | null {
  const m = html.match(/__REACT_QUERY_STATE__\s*=\s*(\{.*?\});\s*\n/s)
  if (!m) return null
  try { return JSON.parse(m[1]) as Record<string, unknown> } catch { return null }
}

interface ChapterInfo {
  firstChapter?: { slug: string; name?: string }
  chapterGroups?: { id: number; order: number; counts: { total: number } }[]
}

function extractChapterInfo(html: string): ChapterInfo | null {
  const state = parseReactQueryState(html)
  if (!state) return null
  const queries = state.queries as { queryKey?: unknown[]; state?: { data?: { item?: { chapterInfo?: ChapterInfo } } } }[] | undefined
  const novelQ = queries?.find(q => Array.isArray(q.queryKey) && q.queryKey.includes('novel'))
  return novelQ?.state?.data?.item?.chapterInfo ?? null
}

interface ChapterSSRData {
  content: string | null
  nextSlug: string | null
  title: string | null
}

function extractChapterSSR(html: string): ChapterSSRData | null {
  const state = parseReactQueryState(html)
  if (!state) return null
  const queries = state.queries as { queryKey?: unknown[]; state?: { data?: { item?: Record<string, unknown> } } }[] | undefined
  const q = queries?.find(q => Array.isArray(q.queryKey) && q.queryKey[0] === 'chapter')
  const item = q?.state?.data?.item
  if (!item) return null
  const contentVal = (item.content as { value?: string } | null)?.value ?? null
  const nextSlug = (item.relatedChapterInfo as { nextChapter?: { slug?: string } } | null)?.nextChapter?.slug ?? null
  const title = (item.name as string | undefined) ?? null
  return { content: contentVal, nextSlug, title }
}

// ── Chapters ──────────────────────────────────────────────────────────────────
// Fast path: one request to /chapters page, read chapter counts from chapterGroups.
// Ch.1 gets the real slug from firstChapter. Chapters 2+ get numeric placeholder
// source_ids (novelSlug/chapter/N) that chapterText() resolves lazily via slug cache.

export async function chapters(novelSlug: string): Promise<ChapterItem[]> {
  const html = await getHtml(`${BASE}/novel/${novelSlug}/chapters`)
  const info = extractChapterInfo(html)
  if (!info || !info.chapterGroups?.length) return []

  const { firstChapter, chapterGroups } = info
  const result: ChapterItem[] = []
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

// ── Slug cache ────────────────────────────────────────────────────────────────
// Chapters 2+ have numeric source_ids. To download them we need the real slug.
// We traverse the linked list (relatedChapterInfo.nextChapter.slug) lazily and
// cache results in memory. Sequential downloads are O(1) amortized: each download
// extends the chain by one step and caches it for the next.
//
// Cache key: novelSlug → ordered array of slugs (index 0 = ch.1 slug)
// The array grows as chapters are resolved. Thread-safety: Node.js is
// single-threaded so array operations are atomic within a turn of the event loop.

const slugChain = new Map<string, string[]>()
export function _clearSlugCache(): void { slugChain.clear() }

async function resolveSlug(novelSlug: string, targetNum: number): Promise<string> {
  // Bootstrap: fetch ch.1 slug from the /chapters page SSR
  if (!slugChain.has(novelSlug)) {
    const html = await getHtml(`${BASE}/novel/${novelSlug}/chapters`)
    const info = extractChapterInfo(html)
    if (!info?.firstChapter?.slug) throw new Error(`wuxiaworld: no firstChapter for ${novelSlug}`)
    slugChain.set(novelSlug, [info.firstChapter.slug])
  }

  const chain = slugChain.get(novelSlug)!

  // Traverse from current end until we reach targetNum
  while (chain.length < targetNum) {
    const currentSlug = chain[chain.length - 1]
    const html = await getHtml(`${BASE}/novel/${novelSlug}/${currentSlug}`)
    const data = extractChapterSSR(html)
    if (!data?.nextSlug) throw new Error(`wuxiaworld: chain ended at ch.${chain.length} (target ${targetNum})`)
    chain.push(data.nextSlug)
  }

  return chain[targetNum - 1]
}

// ── Chapter text ──────────────────────────────────────────────────────────────
// source_id formats:
//   slug:    "novelSlug/chapterSlug"     → direct fetch, content in SSR
//   numeric: "novelSlug/chapter/{N}"    → resolve real slug via cache, then fetch

const NUMERIC_RE = /^([^/]+)\/chapter\/(\d+)$/

export async function chapterText(chapterId: string): Promise<string> {
  let url = `${BASE}/novel/${chapterId}`

  const m = chapterId.match(NUMERIC_RE)
  if (m) {
    const novelSlug = m[1]
    const num = parseInt(m[2], 10)
    const realSlug = await resolveSlug(novelSlug, num)
    url = `${BASE}/novel/${novelSlug}/${realSlug}`
  }

  const html = await getHtml(url)
  const data = extractChapterSSR(html)
  if (data?.content) {
    const text = td.turndown(data.content).trim()
    if (text) return text
  }
  throw new Error(`wuxiaworld: no chapter content at ${url}`)
}
