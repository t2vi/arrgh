const BASE = 'https://api.comick.dev'

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

// ── JSON fetch via stealth browser (bypasses CF TLS fingerprinting) ──────────

// Browsers render JSON APIs as <html><body><pre>JSON</pre></body></html>.
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
  const browser = await _ctx!.getBrowser()
  const bctx = await browser.newContext()
  const page = await bctx.newPage()
  try {
    await page.goto(url, { waitUntil: 'networkidle', timeout: 60_000 })
    const html = await page.content()
    const extracted = extractJson(html)
    if (!extracted.trimStart().match(/^[\[{]/)) {
      throw new Error(`unexpected response from ${url}: ${extracted.slice(0, 100)}`)
    }
    return JSON.parse(extracted) as T
  } finally {
    await bctx.close()
  }
}

// ── Wire types ────────────────────────────────────────────────────────────────

interface Cover { b2key: string }
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
  chapters: ChapterEntry[] | null
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
    id: c.hid,
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

    const chapterList = Array.isArray(data.chapters) ? data.chapters : []
    if (!Array.isArray(data.chapters)) {
      console.warn(`[comick] unexpected chapters shape for "${slug}" page ${page}:`, JSON.stringify(data).slice(0, 200))
    }

    for (const ch of chapterList) {
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

    if (page * LIMIT >= data.total || chapterList.length === 0) break
    page++
  }

  return Array.from(seen.values()).sort((a, b) => a.number - b.number)
}

export async function pages(chapterHid: string): Promise<string[]> {
  const data = await fetchComick<ChapterDetailResponse>(`/chapter/${chapterHid}`)
  return data.chapter.md_images.map((img) => img.url)
}

export async function cover(url: string): Promise<Buffer> {
  const browser = await _ctx!.getBrowser()
  const bctx = await browser.newContext()
  const page = await bctx.newPage()
  try {
    const b64: string = await page.evaluate(async (imgUrl: string) => {
      const resp = await fetch(imgUrl, {
        headers: { Referer: 'https://comick.io/', Accept: 'image/*,*/*' },
      })
      if (!resp.ok) throw new Error(`HTTP ${resp.status}`)
      const ab = await resp.arrayBuffer()
      const bytes = new Uint8Array(ab)
      let str = ''
      bytes.forEach((b) => { str += String.fromCharCode(b) })
      return btoa(str)
    }, url)
    return Buffer.from(b64, 'base64')
  } finally {
    await bctx.close()
  }
}
