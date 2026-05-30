import * as cheerio from 'cheerio'
import TurndownService from 'turndown'

const BASE = 'https://boxnovel.com'
const UA = 'Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/124.0.0.0 Safari/537.36'
const td = new TurndownService({ headingStyle: 'atx', bulletListMarker: '-' })

async function getHtml(url: string): Promise<string> {
  const res = await fetch(url, { headers: { 'User-Agent': UA } })
  if (!res.ok) throw new Error(`boxnovel: ${url} returned ${res.status}`)
  return res.text()
}

function extractSlug(href: string): string {
  // https://boxnovel.com/novel/lord-of-the-mysteries/ → lord-of-the-mysteries
  return href.replace(/\/$/, '').split('/').pop() ?? ''
}

function normalizeStatus(s: string): string {
  return s.toLowerCase().trim()
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
// BoxNovel is WordPress-based; search: /?s=query&post_type=wp-manga

export async function search(query: string): Promise<SearchItem[]> {
  const q = encodeURIComponent(query)
  const html = await getHtml(`${BASE}/?s=${q}&post_type=wp-manga`)
  const $ = cheerio.load(html)
  const results: SearchItem[] = []
  const seen = new Set<string>()

  // WordPress manga theme: .c-tabs-item
  $('.c-tabs-item').each((_, el) => {
    const $el = $(el)
    const $link = $el.find('a[href*="/novel/"]').first()
    const href = $link.attr('href') ?? ''
    const id = extractSlug(href)
    if (!id || seen.has(id)) return
    seen.add(id)

    const title = $el.find('.post-title h5 a, .post-title a').first().text().trim()
      || $el.find('img').first().attr('alt')?.trim()
      || ''
    if (!title) return

    const cover = $el.find('img.lazy, img.img-responsive').first().attr('src')
      || $el.find('img').first().attr('src')
      || null

    // Status and author from .post-content_item rows
    let status = 'unknown'
    let author: string | null = null
    $el.find('.post-content_item').each((_, item) => {
      const heading = $(item).find('.summary-heading h5').text().trim().toLowerCase()
      const content = $(item).find('.summary-content').text().trim()
      if (heading.includes('status')) status = normalizeStatus(content)
      if (heading.includes('author')) author = content || null
    })

    results.push({
      id,
      title,
      description: null,
      cover_url: cover,
      status,
      author,
      year: null,
      tags: null,
      content_type: 'novel',
    })
  })

  return results
}

// ── Chapters ──────────────────────────────────────────────────────────────────
// BoxNovel chapter list on the novel page: ul.main.version-chap li.wp-manga-chapter

export async function chapters(novelId: string): Promise<ChapterItem[]> {
  const html = await getHtml(`${BASE}/novel/${novelId}/`)
  const $ = cheerio.load(html)
  const seen = new Map<number, ChapterItem>()

  $('li.wp-manga-chapter a[href*="/chapter-"]').each((_, el) => {
    const $a = $(el)
    const href = $a.attr('href') ?? ''
    const m = href.match(/chapter-(\d+(?:\.\d+)?)/)
    const num = m ? parseFloat(m[1]) : NaN
    if (isNaN(num)) return

    const sourceId = href.startsWith(BASE) ? href.slice(BASE.length).replace(/\/$/, '') : href.replace(/\/$/, '')
    const rawTitle = $a.text().trim() || null
    const item: ChapterItem = { source_id: sourceId, number: num, volume: null, title: rawTitle }
    if (!seen.has(num)) seen.set(num, item)
  })

  return Array.from(seen.values()).sort((a, b) => a.number - b.number)
}

// ── Chapter text ──────────────────────────────────────────────────────────────

export async function chapterText(chapterId: string): Promise<string> {
  const url = chapterId.startsWith('http') ? chapterId : `${BASE}${chapterId.startsWith('/') ? '' : '/'}${chapterId}`
  const html = await getHtml(url)
  const $ = cheerio.load(html)
  const content = $('.reading-content .text-left').html()
    || $('.reading-content').html()
    || ''
  return td.turndown(content).trim()
}
