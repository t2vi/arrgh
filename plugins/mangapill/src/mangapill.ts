import * as cheerio from 'cheerio'

const BASE = 'https://mangapill.com'
const UA = 'Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/124.0.0.0 Safari/537.36'

async function getPage(url: string): Promise<string> {
  const res = await fetch(url, { headers: { 'User-Agent': UA } })
  if (!res.ok) throw new Error(`mangapill: ${url} returned ${res.status}`)
  return res.text()
}

export async function fetchCoverBytes(url: string): Promise<Buffer> {
  const res = await fetch(url, { headers: { 'User-Agent': UA, 'Referer': BASE } })
  if (!res.ok) throw new Error(`cover fetch failed: ${res.status}`)
  return Buffer.from(await res.arrayBuffer())
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

export interface MetaResponse {
  description: string | null
  cover_url: string | null
  chapter_count: number
  tags: string | null
}

export async function search(query: string): Promise<SearchItem[]> {
  const q = query.split('').map(c => /[a-zA-Z0-9\-_.]/.test(c) ? c : c === ' ' ? '+' : '_').join('')
  const html = await getPage(`${BASE}/search?q=${q}&type=&status=`)
  const $ = cheerio.load(html)
  const results: SearchItem[] = []
  const seen = new Set<string>()

  $('div.my-3 > div').each((_, el) => {
    const $el = $(el)
    const $link = $el.find('a[href^="/manga/"]').first()
    const href = $link.attr('href') ?? ''
    const id = href.replace(/^\/manga\//, '')
    if (!id || seen.has(id)) return
    seen.add(id)

    const title = $el.find('div.font-black').first().text().trim()
    if (!title) return

    const cover_url = $el.find('img[data-src]').first().attr('data-src') ?? null

    const status = $el.find('div.bg-green-500, div.bg-red-500, div.bg-gray-500').first().text().trim().toLowerCase() || 'unknown'

    const yearText = $el.find('div.bg-orange-500').first().text().trim()
    const year = yearText ? parseInt(yearText, 10) || null : null

    const tags = $el.find('div.bg-card').map((_, t) => $(t).text().trim()).get().filter(Boolean).slice(0, 8)

    results.push({
      id,
      title,
      description: null,
      cover_url,
      status,
      author: null,
      year,
      tags: tags.length ? tags.join(', ') : null,
      content_type: 'manga',
    })
  })

  return results
}

export async function trending(): Promise<SearchItem[]> {
  const html = await getPage(BASE)
  const $ = cheerio.load(html)
  const results: SearchItem[] = []
  const seen = new Set<string>()

  $('div.my-3.grid > div').each((_, el) => {
    const $el = $(el)
    const $link = $el.find('a[href^="/manga/"]').first()
    const href = $link.attr('href') ?? ''
    const id = href.replace(/^\/manga\//, '')
    if (!id || seen.has(id)) return
    seen.add(id)

    const title = $el.find('div.font-black').first().text().trim()
    if (!title) return

    const cover_url = $el.find('img[data-src]').first().attr('data-src') ?? null

    const chips = $el.find('div.bg-card.rounded').map((_, c) => $(c).text().trim()).get().filter(Boolean)
    const year = chips.find(s => /^\d{4}$/.test(s) && parseInt(s) > 1900) ? parseInt(chips.find(s => /^\d{4}$/.test(s))!) : null
    const status = chips.find(s => ['ongoing', 'completed', 'cancelled', 'hiatus', 'publishing'].includes(s)) ?? 'unknown'

    results.push({
      id,
      title,
      description: null,
      cover_url,
      status,
      author: null,
      year,
      tags: null,
      content_type: 'manga',
    })

    if (results.length >= 20) return false
  })

  if (results.length === 0) throw new Error('mangapill: no trending found on homepage')
  return results
}

export async function meta(sourceId: string): Promise<MetaResponse> {
  const html = await getPage(`${BASE}/manga/${sourceId}`)
  const $ = cheerio.load(html)

  const description = $('p.text-sm.text--secondary').first().text().trim() || null
  const cover_url = $('img[data-src]').first().attr('data-src') ?? null
  const chapter_count = $('a[href^="/chapters/"]').length

  return { description, cover_url: cover_url ?? null, chapter_count, tags: null }
}

export async function chapters(sourceId: string): Promise<ChapterItem[]> {
  const html = await getPage(`${BASE}/manga/${sourceId}`)
  const $ = cheerio.load(html)
  const items: ChapterItem[] = []

  $('a[href^="/chapters/"]').each((_, el) => {
    const href = $(el).attr('href') ?? ''
    const chapterSourceId = href.replace(/^\/chapters\//, '').replace(/\/$/, '')
    if (!chapterSourceId) return
    const number = parseChapterNumber(chapterSourceId)
    items.push({ source_id: chapterSourceId, number, volume: null, title: null })
  })

  return items
}

export async function pages(chapterSourceId: string): Promise<{ url: string; referer: string }[]> {
  const html = await getPage(`${BASE}/chapters/${chapterSourceId}`)
  const $ = cheerio.load(html)
  const results: { url: string; referer: string }[] = []

  $('img[data-src]').each((_, el) => {
    const src = $(el).attr('data-src')?.trim()
    if (src) results.push({ url: src, referer: BASE })
  })

  if (results.length === 0) throw new Error(`no images found for chapter ${chapterSourceId}`)
  return results
}

// source_id: "2-11182000/one-piece-chapter-1182" → 1182
export function parseChapterNumber(sourceId: string): number {
  const slug = sourceId.split('/')[1] ?? ''
  const fromSlug = slug.match(/chapter-(\d+(?:\.\d+)?)/)
  if (fromSlug) return parseFloat(fromSlug[1]!)

  const idPart = sourceId.split('/')[0] ?? ''
  const encoded = parseInt(idPart.split('-')[1] ?? '', 10)
  if (!isNaN(encoded)) {
    const n = (encoded - 10_000_000) / 1000
    if (n >= 0) return n
  }
  return 0
}
