import * as cheerio from 'cheerio'
import TurndownService from 'turndown'

const BASE = 'https://novelfull.com'
const FLARESOLVERR = () => process.env.FLARESOLVERR_URL ?? 'http://flaresolverr:8191'

const td = new TurndownService({ headingStyle: 'atx', hr: '---', bulletListMarker: '-' })

// ── FlareSolverr ─────────────────────────────────────────────────────────────

interface FlareSolverrResult {
  status: string
  solution: { response: string; status: number }
}

async function flareHtml(url: string): Promise<string> {
  const res = await fetch(`${FLARESOLVERR()}/v1`, {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify({ cmd: 'request.get', url, maxTimeout: 60000 }),
  })
  if (!res.ok) throw new Error(`FlareSolverr HTTP ${res.status}`)
  const data = (await res.json()) as FlareSolverrResult
  if (data.status !== 'ok') throw new Error(`FlareSolverr error: ${data.status}`)
  if (data.solution.status !== 200) throw new Error(`NovelFull ${data.solution.status} for ${url}`)
  return data.solution.response
}

// ── Types ─────────────────────────────────────────────────────────────────────

export interface SearchResult {
  id: string
  title: string
  description: string | null
  cover_url: string | null
  status: string
  author: string | null
  year: number | null
  tags: string | null
}

export interface ChapterResult {
  source_id: string
  number: number
  title: string | null
  chapter_format: 'text'
}

export interface MetaResult {
  description: string | null
  cover_url: string | null
  chapter_count: number
  tags: string | null
}

// ── Helpers ───────────────────────────────────────────────────────────────────

// source_id = slug without ".html", e.g. "i-shall-seal-the-heavens"
// chapter source_id = relative path, e.g. "i-shall-seal-the-heavens/chapter-1.html"

function absUrl(path: string): string {
  if (path.startsWith('http')) return path
  return `${BASE}/${path.replace(/^\//, '')}`
}

function coverUrl(src: string | undefined): string | null {
  if (!src) return null
  return absUrl(src)
}

async function fetchChapterPage(slug: string, page: number): Promise<cheerio.CheerioAPI> {
  const url = `${BASE}/${slug}.html${page > 1 ? `?page=${page}` : ''}`
  const html = await flareHtml(url)
  return cheerio.load(html)
}

// Fetch all chapter pages concurrently (max 4 at a time)
async function fetchAllChapters(slug: string): Promise<ChapterResult[]> {
  const $ = await fetchChapterPage(slug, 1)

  // Parse chapter links from first page
  const firstBatch = parseChapterLinks($)

  // Detect total pages from pagination
  const lastPage = parseInt($('.pagination li:last-child a').attr('data-page') ?? '1', 10) || 1

  if (lastPage <= 1) return firstBatch

  // Fetch remaining pages concurrently (up to 4 at a time)
  const pageNums = Array.from({ length: lastPage - 1 }, (_, i) => i + 2)
  const batches: ChapterResult[][] = [firstBatch]

  for (let i = 0; i < pageNums.length; i += 4) {
    const chunk = pageNums.slice(i, i + 4)
    const pages = await Promise.all(chunk.map(async (p) => {
      const $p = await fetchChapterPage(slug, p)
      return parseChapterLinks($p)
    }))
    batches.push(...pages)
  }

  const all = batches.flat()
  // Re-number sequentially (page order = chapter order)
  return all.map((c, i) => ({ ...c, number: i + 1 }))
}

function parseChapterLinks($: cheerio.CheerioAPI): ChapterResult[] {
  const results: ChapterResult[] = []
  $('#list-chapter .row li a, .list-chapter li a').each((i, el) => {
    const href = $(el).attr('href') ?? ''
    // href = "/i-shall-seal-the-heavens/chapter-1-my-life-begins.html"
    const path = href.replace(/^\//, '')
    if (!path || !path.includes('/chapter')) return
    const title = $(el).text().trim() || null
    results.push({ source_id: path, number: i + 1, title, chapter_format: 'text' })
  })
  return results
}

// ── Public API ────────────────────────────────────────────────────────────────

export async function search(query: string): Promise<SearchResult[]> {
  const url = `${BASE}/search?keyword=${encodeURIComponent(query)}`
  const html = await flareHtml(url)
  const $ = cheerio.load(html)
  const results: SearchResult[] = []

  $('.list-truyen .row, .truyen-list .row').each((_, el) => {
    const $el = $(el)
    const linkEl = $el.find('.truyen-title a, h3.truyen-title a').first()
    const href = linkEl.attr('href') ?? ''
    // href = "/i-shall-seal-the-heavens.html"
    const slug = href.replace(/^\//, '').replace(/\.html$/, '')
    if (!slug) return

    const title = linkEl.text().trim()
    if (!title) return

    const imgEl = $el.find('img').first()
    const cover = coverUrl(imgEl.attr('data-src') ?? imgEl.attr('src'))
    const author = $el.find('.author').text().trim() || null

    results.push({
      id: slug,
      title,
      description: null,
      cover_url: cover,
      status: 'ongoing',
      author,
      year: null,
      tags: null,
    })
  })

  return results
}

export async function chapters(slug: string): Promise<ChapterResult[]> {
  return fetchAllChapters(slug)
}

export async function chapterText(chapterPath: string): Promise<string> {
  // chapterPath = "i-shall-seal-the-heavens/chapter-1-my-life-begins.html"
  const url = `${BASE}/${chapterPath}`
  const html = await flareHtml(url)
  const $ = cheerio.load(html)

  const contentEl = $('#chapter-content').first()
  if (!contentEl.length) throw new Error(`novelfull: no chapter content at ${url}`)

  // Remove ads, navigation ads, and scripts embedded in content
  contentEl.find('.ads, .ads-holder, script, ins, [class*="ads"]').remove()

  return td.turndown(contentEl.html() ?? '')
}

export async function meta(slug: string): Promise<MetaResult> {
  const url = `${BASE}/${slug}.html`
  const html = await flareHtml(url)
  const $ = cheerio.load(html)

  const desc = $('.desc-text p').map((_, el) => $(el).text().trim()).get().join('\n\n') || null
  const imgEl = $('.book img').first()
  const cover = coverUrl(imgEl.attr('data-src') ?? imgEl.attr('src'))

  // Chapter count from total pages × 50 (approx) or direct count from first page list
  const totalPageStr = $('.pagination li:last-child a').attr('data-page') ?? '1'
  const totalPages = parseInt(totalPageStr, 10) || 1
  const firstPageCount = $('#list-chapter .row li a').length
  const chapterCount = (totalPages - 1) * 50 + firstPageCount

  const tags = $('.info-holder .info li:last-child a')
    .map((_, el) => $(el).text().trim())
    .get()
    .filter(Boolean)
    .join(', ') || null

  return { description: desc, cover_url: cover, chapter_count: chapterCount, tags }
}
