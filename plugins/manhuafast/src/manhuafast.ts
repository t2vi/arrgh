import * as cheerio from 'cheerio'

const BASE = 'https://manhuafast.net'
const UA = 'Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/124.0.0.0 Safari/537.36'

async function getHtml(url: string): Promise<string> {
  const res = await fetch(url, { headers: { 'User-Agent': UA } })
  if (!res.ok) throw new Error(`manhuafast: ${url} returned ${res.status}`)
  return res.text()
}

function normalizeStatus(s: string): string {
  return s.toLowerCase().trim().replace('ongoing', 'ongoing')
}

function extractSlug(href: string): string {
  // https://manhuafast.net/manga/the-beginning-after-the-end/ → the-beginning-after-the-end
  return href.replace(/\/$/, '').split('/').pop() ?? ''
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
// ManhuaFast is WordPress-based; search: /search/?s=query

export async function search(query: string): Promise<SearchItem[]> {
  const q = encodeURIComponent(query)
  const html = await getHtml(`${BASE}/?s=${q}&post_type=wp-manga`)
  const $ = cheerio.load(html)
  const results: SearchItem[] = []
  const seen = new Set<string>()

  // WordPress manga theme: .page-listing-item or .c-tabs-item
  $('.page-listing-item, .c-tabs-item').each((_, el) => {
    const $el = $(el)
    const $link = $el.find('a[href*="/manga/"]').first()
    const href = $link.attr('href') ?? ''
    const id = extractSlug(href)
    if (!id || seen.has(id)) return
    seen.add(id)

    const title = $el.find('.post-title h5 a, .item-title a, h5 a').first().text().trim()
      || $el.find('img').first().attr('alt')?.trim()
      || ''
    if (!title) return

    const cover = $el.find('img.lazy, img.img-responsive').first().attr('src')
      || $el.find('img').first().attr('src')
      || null
    const statusRaw = $el.find('.manga-title-badges span, .summary-content').first().text().trim()

    results.push({
      id,
      title,
      description: null,
      cover_url: cover,
      status: normalizeStatus(statusRaw || 'ongoing'),
      author: null,
      year: null,
      tags: null,
      content_type: 'manhua',
    })
  })

  return results
}

// ── Chapters ──────────────────────────────────────────────────────────────────
// WordPress chapter list via AJAX POST or on the series page

export async function chapters(seriesId: string): Promise<ChapterItem[]> {
  const html = await getHtml(`${BASE}/manga/${seriesId}/`)
  const $ = cheerio.load(html)
  const all: ChapterItem[] = []
  const seen = new Map<number, ChapterItem>()

  // WordPress manga theme chapter list: .wp-manga-chapter li
  $('li.wp-manga-chapter a[href*="/chapter-"]').each((_, el) => {
    const $a = $(el)
    const href = $a.attr('href') ?? ''
    const m = href.match(/chapter-(\d+(?:\.\d+)?)/)
    const num = m ? parseFloat(m[1]) : NaN
    if (isNaN(num)) return

    // source_id = URL path after BASE
    const sourceId = href.startsWith(BASE) ? href.slice(BASE.length).replace(/\/$/, '') : href.replace(/\/$/, '')
    const item: ChapterItem = { source_id: sourceId, number: num, volume: null, title: null }
    if (!seen.has(num)) seen.set(num, item)
  })

  return Array.from(seen.values()).sort((a, b) => a.number - b.number)
}

// ── Pages ─────────────────────────────────────────────────────────────────────

export async function pages(chapterId: string): Promise<string[]> {
  const url = chapterId.startsWith('http') ? chapterId : `${BASE}${chapterId.startsWith('/') ? '' : '/'}${chapterId}`
  const html = await getHtml(url)
  const $ = cheerio.load(html)

  const urls: string[] = []
  // WordPress manga reader: .reading-content .page-break img
  $('.reading-content img.wp-manga-chapter-img, .reading-content .page-break img').each((_, el) => {
    const src = $(el).attr('src')?.trim()
    if (src && src.startsWith('http')) urls.push(src)
  })

  return urls
}
