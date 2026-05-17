const BASE = 'https://api.comick.fun'
const FLARESOLVERR = () => process.env.FLARESOLVERR_URL ?? 'http://flaresolverr:8191'

// ── FlareSolverr client ───────────────────────────────────────────────────────

interface FlareSolverrResult {
  status: string
  solution: { response: string; status: number }
}

// Browsers render JSON APIs as <html><body><pre>JSON</pre></body></html>.
// FlareSolverr uses a real browser, so we must unwrap the <pre> content.
function extractJson(html: string): string {
  const match = html.match(/<pre[^>]*>([\s\S]*?)<\/pre>/i)
  const raw = match?.[1] ?? html
  return raw
    .replace(/&amp;/g, '&')
    .replace(/&lt;/g, '<')
    .replace(/&gt;/g, '>')
    .replace(/&quot;/g, '"')
    .replace(/&#39;/g, "'")
}

async function flareFetch<T>(url: string): Promise<T> {
  const res = await fetch(`${FLARESOLVERR()}/v1`, {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify({ cmd: 'request.get', url, maxTimeout: 60000 }),
  })
  if (!res.ok) throw new Error(`FlareSolverr HTTP ${res.status}`)
  const data = (await res.json()) as FlareSolverrResult
  if (data.status !== 'ok') throw new Error(`FlareSolverr error: ${data.status}`)
  if (data.solution.status !== 200) throw new Error(`Comick ${data.solution.status} for ${url}`)
  return JSON.parse(extractJson(data.solution.response)) as T
}

// ── Wire types ────────────────────────────────────────────────────────────────

interface Cover { b2key: string }
// Search results return genres as numeric IDs; detail endpoint returns objects.
type Genre = number | { name: string }
interface Author { name: string }

interface ComicResult {
  hid: string
  slug: string
  title: string
  country: string | null
  content_rating: string
  status: number | null
  year: number | null
  md_covers: Cover[]
  chapter_count?: number
  genres?: Genre[]
  authors?: Author[]
  desc?: string | null
}

interface ChapterEntry {
  hid: string
  chap: string | null
  vol: string | null
  title: string | null
  lang: string
}

interface ChaptersResponse {
  chapters: ChapterEntry[]
  total: number
}

interface ComicDetailResponse {
  comic: ComicResult
  authors: Author[]
  artists: Author[]
}

interface ChapterDetailResponse {
  chapter: {
    md_images: Array<{ url: string }>
  }
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

export interface MetaResult {
  description: string | null
  cover_url: string | null
  chapter_count: number
  tags: string | null
}

// ── Helpers ───────────────────────────────────────────────────────────────────

function coverUrl(covers: Cover[]): string | null {
  const key = covers[0]?.b2key
  return key ? `https://meo.comick.pictures/${key}` : null
}

function statusStr(n: number | null): string {
  switch (n) {
    case 1: return 'ongoing'
    case 2: return 'completed'
    case 3: return 'cancelled'
    case 4: return 'hiatus'
    default: return 'unknown'
  }
}

const EXPLICIT_LABELS = new Set(['adult', 'mature', 'hentai', 'smut', '18+'])

function buildTags(content_rating: string, genres?: Genre[]): string | null {
  const tags: string[] = []
  if (content_rating === 'erotica') tags.push('adult')
  if (genres) {
    for (const g of genres) {
      if (typeof g !== 'object' || !g.name) continue
      if (EXPLICIT_LABELS.has(g.name.toLowerCase())) {
        if (!tags.includes('adult')) tags.push('adult')
      } else {
        tags.push(g.name)
      }
    }
  }
  return tags.length ? tags.join(', ') : null
}

function toSearchItem(c: ComicResult, authors?: Author[]): SearchItem {
  const author = (c.authors ?? authors ?? [])[0]?.name ?? null
  return {
    id: c.slug,
    title: c.title,
    description: c.desc ?? null,
    cover_url: coverUrl(c.md_covers),
    status: statusStr(c.status),
    author,
    year: c.year ?? null,
    tags: buildTags(c.content_rating, c.genres),
  }
}

// ── Client ────────────────────────────────────────────────────────────────────

async function fetchComick<T>(path: string, params?: Record<string, string>): Promise<T> {
  const url = new URL(BASE + path)
  if (params) Object.entries(params).forEach(([k, v]) => url.searchParams.set(k, v))
  return flareFetch<T>(url.toString())
}

// ── Endpoints ─────────────────────────────────────────────────────────────────

export async function search(query: string): Promise<SearchItem[]> {
  const data = await fetchComick<ComicResult[]>('/v1.0/search', { q: query, limit: '20' })
  return data.map((c) => toSearchItem(c))
}

export async function trending(): Promise<SearchItem[]> {
  // sort=follow = most followed — stable popular titles per the Comick API
  const data = await fetchComick<ComicResult[]>('/v1.0/search', { sort: 'follow', limit: '20' })
  return data.map((c) => toSearchItem(c))
}

export async function meta(slug: string): Promise<MetaResult> {
  const data = await fetchComick<ComicDetailResponse>(`/comic/${slug}`)
  const c = data.comic
  return {
    description: c.desc ?? null,
    cover_url: coverUrl(c.md_covers),
    chapter_count: c.chapter_count ?? 0,
    tags: buildTags(c.content_rating, c.genres),
  }
}

export async function chapters(slug: string, langs: string[]): Promise<ChapterItem[]> {
  const LIMIT = 100
  let page = 1
  const seen = new Map<number, ChapterItem>()

  while (true) {
    const data = await fetchComick<ChaptersResponse>(`/comic/${slug}/chapters`, {
      page: String(page),
      limit: String(LIMIT),
      lang: langs[0] ?? 'en',
    })

    for (const ch of data.chapters) {
      const num = parseFloat(ch.chap ?? '')
      if (isNaN(num)) continue
      if (!seen.has(num)) {
        seen.set(num, {
          source_id: ch.hid,
          number: num,
          volume: ch.vol ? parseFloat(ch.vol) : null,
          title: ch.title ?? null,
        })
      }
    }

    if (page * LIMIT >= data.total || data.chapters.length === 0) break
    page++
  }

  return Array.from(seen.values()).sort((a, b) => a.number - b.number)
}

export async function pages(chapterHid: string): Promise<string[]> {
  const data = await fetchComick<ChapterDetailResponse>(`/chapter/${chapterHid}`)
  return data.chapter.md_images.map((img) => img.url)
}
