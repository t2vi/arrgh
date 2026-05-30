import * as cheerio from 'cheerio'

const BASE = 'https://asuracomic.net'
const UA = 'Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/124.0.0.0 Safari/537.36'

async function getHtml(url: string): Promise<string> {
  const res = await fetch(url, { headers: { 'User-Agent': UA } })
  if (!res.ok) throw new Error(`asurascans: ${url} returned ${res.status}`)
  return res.text()
}

function normalizeStatus(s: string): string {
  return s.toLowerCase().trim()
    .replace('hiatus', 'hiatus')
    .replace('dropped', 'dropped')
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
// AsuraScans search: https://asuracomic.net/?s=query

export async function search(query: string): Promise<SearchItem[]> {
  const q = encodeURIComponent(query)
  const html = await getHtml(`${BASE}/?s=${q}`)
  const $ = cheerio.load(html)
  const results: SearchItem[] = []
  const seen = new Set<string>()

  // Asura uses Tailwind-styled series grid with group/tipmanga items
  $('a[href*="/series/"]').each((_, el) => {
    const $a = $(el)
    const href = $a.attr('href') ?? ''
    // href: https://asuracomic.net/series/solo-leveling-abc123
    const m = href.match(/\/series\/([\w-]+)\/?$/)
    const id = m?.[1]
    if (!id || seen.has(id)) return
    seen.add(id)

    // Title: span.font-bold, or img alt
    const title = $a.find('span.font-bold, .tt').first().text().trim()
      || $a.find('img').first().attr('alt')?.trim()
      || ''
    if (!title) return

    const cover = $a.find('img').first().attr('src') || null
    const statusRaw = $a.find('.status, [class*="status"]').first().text().trim() || 'ongoing'
    // AsuraScans is manhwa-only
    const contentType = $a.find('span.text-xs').eq(0).text().trim().toLowerCase() || 'manhwa'

    results.push({
      id,
      title,
      description: null,
      cover_url: cover,
      status: normalizeStatus(statusRaw),
      author: null,
      year: null,
      tags: null,
      content_type: contentType === 'manhwa' ? 'manhwa' : 'manhwa',
    })
  })

  return results
}

// ── Chapters ──────────────────────────────────────────────────────────────────

export async function chapters(seriesId: string): Promise<ChapterItem[]> {
  const html = await getHtml(`${BASE}/series/${seriesId}`)
  const $ = cheerio.load(html)
  const all: ChapterItem[] = []
  const seen = new Map<number, ChapterItem>()

  // Chapter items: div[data-num] or li[data-num] with links
  $('[data-num] a[href*="/chapter/"], a[href*="/chapter/"]').each((_, el) => {
    const $a = $(el)
    const href = $a.attr('href') ?? ''
    const $parent = $a.parents('[data-num]').first()
    const numAttr = $parent.attr('data-num')

    let num: number
    if (numAttr) {
      num = parseFloat(numAttr)
    } else {
      const m = href.match(/\/chapter\/(\d+(?:\.\d+)?)/)
      num = m ? parseFloat(m[1]) : NaN
    }
    if (isNaN(num)) return

    // source_id = full path from BASE (e.g. /series/slug/chapter/180)
    const sourceId = href.startsWith(BASE) ? href.slice(BASE.length) : href
    if (!sourceId) return

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
  // AsuraScans renders images in flex column containers
  $('img[src*="asuracomic"], img[src*="gg.asura"], .object-cover[src]').each((_, el) => {
    const src = $(el).attr('src')
    if (src && src.startsWith('http')) urls.push(src)
  })

  // Fallback: collect all chapter page images
  if (urls.length === 0) {
    $('div.flex img[src^="http"], div[class*="reading"] img[src^="http"]').each((_, el) => {
      const src = $(el).attr('src')
      if (src) urls.push(src)
    })
  }

  return urls
}
