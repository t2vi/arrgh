import * as cheerio from 'cheerio'

const BASE = 'https://mangafire.to'
const UA = 'Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/124.0.0.0 Safari/537.36'

async function getHtml(url: string): Promise<string> {
  const res = await fetch(url, { headers: { 'User-Agent': UA } })
  if (!res.ok) throw new Error(`mangafire: ${url} returned ${res.status}`)
  return res.text()
}

async function getJson<T>(url: string, init?: RequestInit): Promise<T> {
  const res = await fetch(url, { headers: { 'User-Agent': UA, 'X-Requested-With': 'XMLHttpRequest', ...init?.headers }, ...init })
  if (!res.ok) throw new Error(`mangafire: ${url} returned ${res.status}`)
  return res.json() as Promise<T>
}

// Type-to-content_type mapping
function mapType(raw: string): string {
  const t = raw.toLowerCase().trim()
  if (t === 'manga')   return 'manga'
  if (t === 'manhwa')  return 'manhwa'
  if (t === 'manhua')  return 'manhua'
  if (t === 'one-shot' || t === 'oneshot') return 'one-shot'
  return 'manga'
}

function normalizeStatus(s: string): string {
  const t = s.toLowerCase().trim()
  if (t === 'publishing' || t === 'ongoing' || t === 'releasing') return 'ongoing'
  if (t === 'completed' || t === 'finished') return 'completed'
  return t
}

// ── Types ─────────────────────────────────────────────────────────────────────

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
  const html = await getHtml(`${BASE}/filter?keyword=${q}`)
  const $ = cheerio.load(html)
  const results: SearchItem[] = []
  const seen = new Set<string>()

  // MangaFire search results: .unit cards in .original.card-list
  $('.unit').each((_, el) => {
    const $el = $(el)
    const $poster = $el.find('a.poster[href]').first()
    const href = $poster.attr('href') ?? ''
    // href like /manga/one-piece-mSME or /manhwa/solo-leveling-mABC
    const match = href.match(/^\/(manga|manhwa|manhua|one-shot)\/(.+)$/)
    if (!match) return
    const [, typeSlug, id] = match
    if (!id || seen.has(id)) return
    seen.add(id)

    const title = $el.find('.info a.name, .info .name').first().text().trim()
      || $el.find('a.poster img').first().attr('alt')?.trim()
      || ''
    if (!title) return

    const typeRaw = $el.find('.info .type, .info span.type').first().text().trim() || typeSlug
    const statusRaw = $el.find('.info .status, .info span.status').first().text().trim()
    const coverEl = $poster.find('img').first()
    const cover = coverEl.attr('src') || coverEl.attr('data-src') || null

    results.push({
      id,
      title,
      description: null,
      cover_url: cover,
      status: normalizeStatus(statusRaw || 'ongoing'),
      author: null,
      year: null,
      tags: null,
      content_type: mapType(typeRaw),
    })
  })

  return results
}

// ── Chapters ──────────────────────────────────────────────────────────────────
// MangaFire loads chapter list via AJAX: GET /ajax/manga/:id/chapter/:lang

export async function chapters(seriesId: string, langs = ['en']): Promise<ChapterItem[]> {
  const lang = langs[0] ?? 'en'
  const data = await getJson<{ html: string }>(`${BASE}/ajax/manga/${seriesId}/chapter/${lang}`)
  const $ = cheerio.load(data.html)
  const all: ChapterItem[] = []
  const seen = new Map<number, ChapterItem>()

  $('ul li[data-number], ul li a[href*="/chapter-"]').each((_, el) => {
    const $li = $(el).is('li') ? $(el) : $(el).parent('li')
    const numAttr = $li.attr('data-number')
    const $a = $li.find('a[href]').first()
    const href = $a.attr('href') ?? ''

    // source_id = path after BASE (e.g. /manga/one-piece-mSME/en/chapter-1050)
    const sourceId = href.startsWith(BASE) ? href.slice(BASE.length) : href

    let num: number
    if (numAttr) {
      num = parseFloat(numAttr)
    } else {
      const m = href.match(/chapter-(\d+(?:\.\d+)?)/)
      num = m ? parseFloat(m[1]) : NaN
    }
    if (isNaN(num) || !sourceId) return

    const title = $a.text().trim() || null
    const item: ChapterItem = { source_id: sourceId, number: num, volume: null, title }
    if (!seen.has(num)) seen.set(num, item)
  })

  return Array.from(seen.values()).sort((a, b) => a.number - b.number)
}

// ── Pages ─────────────────────────────────────────────────────────────────────

export async function pages(chapterId: string): Promise<string[]> {
  const url = chapterId.startsWith('http') ? chapterId : `${BASE}${chapterId.startsWith('/') ? '' : '/'}${chapterId}`
  const html = await getHtml(url)
  const $ = cheerio.load(html)

  // MangaFire renders images in #viewer or .reader-images
  const urls: string[] = []
  $('#viewer img[src], #viewer img[data-src], .reader-images img[src], .reader-images img[data-src]').each((_, el) => {
    const src = $(el).attr('src') || $(el).attr('data-src')
    if (src && src.startsWith('http')) urls.push(src)
  })

  return urls
}
