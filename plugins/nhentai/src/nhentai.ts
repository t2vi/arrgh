const BASE = 'https://nhentai.net'
const API = `${BASE}/api/v2`

// ── Browser interface (duck-typed) ────────────────────────────────────────────

interface BrowserPage {
  goto(url: string, opts?: { waitUntil?: string; timeout?: number }): Promise<unknown>
  content(): Promise<string>
  evaluate<T>(fn: ((...args: unknown[]) => T) | string, ...args: unknown[]): Promise<T>
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

// ── v2 API types ──────────────────────────────────────────────────────────────

interface Gallery {
  id: number
  title: string
}

interface PageInfo {
  path: string  // e.g. "galleries/1322650/1.jpg"
  width: number
  height: number
}

interface GalleryData {
  id: number
  media_id: string
  title: { english: string; japanese: string; pretty: string }
  pages: PageInfo[]
  num_pages: number
}

interface SearchResponse {
  result: Array<{
    id: number
    english_title?: string
    japanese_title?: string
  }>
}

// ── Helpers ───────────────────────────────────────────────────────────────────

function pageImageUrl(path: string): string {
  return `https://i.nhentai.net/${path}`
}

// ── nhentai API via browser context (bypasses CF) ─────────────────────────────

async function withPage<T>(
  landingUrl: string,
  fn: (page: BrowserPage) => Promise<T>
): Promise<T> {
  const browser = await _ctx!.getBrowser()
  const bctx = await browser.newContext()
  const page = await bctx.newPage()
  try {
    await page.goto(landingUrl, { waitUntil: 'load', timeout: 60_000 })
    return await fn(page)
  } finally {
    await bctx.close()
  }
}

async function apiFetch<T>(page: BrowserPage, url: string): Promise<T> {
  return page.evaluate(async (u: string) => {
    const res = await fetch(u, { credentials: 'include' })
    if (!res.ok) throw new Error(`nhentai API ${res.status}: ${u}`)
    return res.json() as Promise<T>
  }, url) as Promise<T>
}

// ── Gallery data ──────────────────────────────────────────────────────────────

async function fetchGalleryData(id: number): Promise<GalleryData> {
  return withPage(`${BASE}/g/${id}/`, async (page) => {
    // Service worker may have cached the API response in an inline script
    const fromCache = await page.evaluate((gid: number) => {
      const w = window as unknown as Record<string, unknown>
      if (w._gallery && (w._gallery as Record<string, unknown>).id === gid) return w._gallery
      for (const s of Array.from(document.querySelectorAll('script:not([src])'))) {
        const t = s.textContent ?? ''
        if (!t.includes('media_id')) continue
        try {
          const obj = JSON.parse(t) as Record<string, unknown>
          const body = typeof obj.body === 'string' ? JSON.parse(obj.body) : obj.body
          if (body && (body as Record<string, unknown>).id === gid) return body
        } catch { /* not a cache entry */ }
      }
      return null
    }, id) as GalleryData | null

    if (fromCache?.pages) return fromCache

    // Fallback: call v2 API directly — CF cookies are set from the page load above
    return apiFetch<GalleryData>(page, `${API}/galleries/${id}`)
  })
}

// ── Search ────────────────────────────────────────────────────────────────────

async function fetchGalleries(query: string): Promise<Gallery[]> {
  const q = encodeURIComponent(`${query} language:english`)
  // Use the search page as landing (sets CF cookies even when it shows the notice page)
  return withPage(`${BASE}/search/?q=${q}`, async (page) => {
    const data = await apiFetch<SearchResponse>(page, `${API}/search?query=${q}&page=1`)
    return (data.result ?? []).map((g) => ({
      id: g.id,
      title: g.english_title ?? g.japanese_title ?? `Gallery ${g.id}`,
    }))
  })
}

// ── Source Plugin Protocol ────────────────────────────────────────────────────

export interface SearchResult {
  id: string
  title: string
  status: string
}

export interface ChapterItem {
  source_id: string
  number: number
  volume: null
  title: string
}

// search() returns the query itself as a virtual series identifier.
// id = URL-encoded query — stored as title_sources.source_id.
export async function search(query: string): Promise<SearchResult[]> {
  const galleries = await fetchGalleries(query)
  if (galleries.length === 0) return []
  return [{ id: encodeURIComponent(query), title: query, status: 'complete' }]
}

// chapters() re-searches and filters by first significant keyword from the query.
export async function chapters(sourceId: string): Promise<ChapterItem[]> {
  const query = decodeURIComponent(sourceId)
  const galleries = await fetchGalleries(query)

  const firstWord = query.toLowerCase()
    .split(/[\s\-_]+/)
    .find((w) => w.length > 2) ?? ''

  const matched = firstWord
    ? galleries.filter((g) => g.title.toLowerCase().includes(firstWord))
    : galleries

  return matched
    .sort((a, b) => a.id - b.id)
    .map((g, i) => ({
      source_id: String(g.id),
      number: i + 1,
      volume: null,
      title: g.title,
    }))
}

// pages() fetches image URLs for a single gallery (source_id = gallery ID string)
export async function pages(galleryId: string): Promise<string[]> {
  const data = await fetchGalleryData(parseInt(galleryId, 10))
  return data.pages.map((p) => pageImageUrl(p.path))
}
