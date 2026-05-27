const BASE = 'https://nhentai.net'

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

// ── Browser fetch ─────────────────────────────────────────────────────────────

async function browserGet(url: string): Promise<string> {
  const browser = await _ctx!.getBrowser()
  const bctx = await browser.newContext()
  const page = await bctx.newPage()
  try {
    await page.goto(url, { waitUntil: 'networkidle', timeout: 60_000 })
    return page.content()
  } finally {
    await bctx.close()
  }
}

// ── HTML parsing helpers ──────────────────────────────────────────────────────

function attr(html: string, selector: RegExp): string | null {
  return html.match(selector)?.[1] ?? null
}

function extractGalleries(html: string): Gallery[] {
  const galleries: Gallery[] = []
  // Each gallery card: <div class="gallery" ...> contains data-id and title
  const cardRe = /<div[^>]+data-id="(\d+)"[^>]*>[\s\S]*?<div[^>]+class="[^"]*caption[^"]*"[^>]*>([\s\S]*?)<\/div>/g
  let m: RegExpExecArray | null
  while ((m = cardRe.exec(html)) !== null) {
    const id = parseInt(m[1], 10)
    const rawTitle = m[2].replace(/<[^>]+>/g, '').trim()
    if (id && rawTitle) galleries.push({ id, title: rawTitle })
  }
  return galleries
}

// ── Wire types ────────────────────────────────────────────────────────────────

interface Gallery {
  id: number
  title: string
}

interface PageInfo {
  t: string  // image type: 'j'=jpeg, 'p'=png, 'w'=webp
  w: number
  h: number
}

interface GalleryData {
  id: number
  media_id: string
  title: { english: string; japanese: string; pretty: string }
  images: { pages: PageInfo[]; cover: PageInfo; thumbnail: PageInfo }
  num_pages: number
}

// ── Output types (Source Plugin Protocol) ────────────────────────────────────

export interface ChapterItem {
  source_id: string
  number: number
  volume: null
  title: string
}

// ── Helpers ───────────────────────────────────────────────────────────────────

function imgExt(t: string): string {
  if (t === 'p') return 'png'
  if (t === 'w') return 'webp'
  return 'jpg'
}

function galleryPageUrl(mediaId: string, page: number, ext: string): string {
  return `https://i.nhentai.net/galleries/${mediaId}/${page}.${ext}`
}

async function fetchGalleryData(id: number): Promise<GalleryData> {
  const html = await browserGet(`${BASE}/api/gallery/${id}`)
  // nhentai /api/gallery/:id returns JSON rendered in a <pre> by the browser
  const preMatch = html.match(/<pre[^>]*>([\s\S]*?)<\/pre>/i)
  const raw = (preMatch?.[1] ?? html)
    .replace(/&amp;/g, '&')
    .replace(/&lt;/g, '<')
    .replace(/&gt;/g, '>')
    .replace(/&quot;/g, '"')
    .replace(/&#39;/g, "'")
  return JSON.parse(raw) as GalleryData
}

// ── Search ────────────────────────────────────────────────────────────────────

export async function search(query: string): Promise<Gallery[]> {
  const q = encodeURIComponent(`${query} language:english`)
  const html = await browserGet(`${BASE}/search/?q=${q}`)
  return extractGalleries(html)
}

// ── Chapters ──────────────────────────────────────────────────────────────────

// source_id = URL-encoded search query (the matched title/alias)
// Re-runs search, keeps galleries whose title starts with the query (case-insensitive),
// sorted by gallery ID ascending → chapter numbering by upload order.
export async function chapters(sourceId: string): Promise<ChapterItem[]> {
  const query = decodeURIComponent(sourceId)
  const galleries = await search(query)
  const prefix = query.toLowerCase()

  const matched = galleries
    .filter((g) => g.title.toLowerCase().startsWith(prefix))
    .sort((a, b) => a.id - b.id)

  return matched.map((g, i) => ({
    source_id: String(g.id),
    number: i + 1,
    volume: null,
    title: g.title,
  }))
}

// ── Pages ─────────────────────────────────────────────────────────────────────

export async function pages(galleryId: string): Promise<string[]> {
  const data = await fetchGalleryData(parseInt(galleryId, 10))
  return data.images.pages.map((p, i) =>
    galleryPageUrl(data.media_id, i + 1, imgExt(p.t))
  )
}
