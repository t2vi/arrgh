import * as cheerio from 'cheerio'
import TurndownService from 'turndown'

const BASE = 'https://www.royalroad.com'

const td = new TurndownService({
  headingStyle: 'atx',
  hr: '---',
  bulletListMarker: '-',
  codeBlockStyle: 'fenced',
})

// Strip author-note asides from chapter text before conversion
td.addRule('remove-author-note', {
  filter: (node) => {
    const el = node as { nodeName?: string; className?: string }
    return el.nodeName === 'DIV' && /author.*note/i.test(el.className ?? '')
  },
  replacement: () => '',
})

async function fetchHtml(url: string): Promise<string> {
  const res = await fetch(url, {
    headers: {
      'User-Agent': 'Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36',
      'Accept-Language': 'en-US,en;q=0.9',
    },
  })
  if (!res.ok) throw new Error(`royalroad: ${url} returned ${res.status}`)
  return res.text()
}

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

// source_id = fiction numeric ID (e.g. "21220")
// Fiction URL: https://www.royalroad.com/fiction/<id> → redirects to /<id>/<slug>
// Chapter source_id = relative path: "fiction/<fid>/<fslug>/chapter/<cid>/<cslug>"

export async function search(query: string): Promise<SearchResult[]> {
  const url = `${BASE}/fictions/search?title=${encodeURIComponent(query)}&orderBy=relevance`
  const html = await fetchHtml(url)
  const $ = cheerio.load(html)
  const results: SearchResult[] = []

  $('.fiction-list-item').each((_, el) => {
    const $el = $(el)
    const linkEl = $el.find('h2.fiction-title a').first()
    const href = linkEl.attr('href') ?? ''
    // href = "/fiction/21220/fiction-slug"
    const match = href.match(/\/fiction\/(\d+)/)
    if (!match) return

    const id = match[1]!
    const title = linkEl.text().trim()
    if (!title) return

    const cover = $el.find('img').first().attr('src') ?? null
    const author = $el.find('.author-name').first().text().trim() || null
    const desc = $el.find('.description').first().text().trim() || null
    const tags = $el.find('.tags .label')
      .map((_, t) => $(t).text().trim())
      .get()
      .filter(Boolean)
      .join(', ') || null

    results.push({ id, title, description: desc, cover_url: cover, status: 'ongoing', author, year: null, tags })
  })

  return results
}

export async function chapters(fictionId: string): Promise<ChapterResult[]> {
  // Follow redirect: /fiction/<id> → /fiction/<id>/<slug>
  const res = await fetch(`${BASE}/fiction/${fictionId}`, {
    headers: { 'User-Agent': 'Mozilla/5.0' },
    redirect: 'follow',
  })
  if (!res.ok) throw new Error(`royalroad: fiction ${fictionId} returned ${res.status}`)
  const finalUrl = res.url  // e.g. https://www.royalroad.com/fiction/21220/wandering-inn
  const html = await res.text()
  const $ = cheerio.load(html)

  // Extract fiction slug from final URL for building chapter source_ids
  const fictionPath = new URL(finalUrl).pathname.slice(1)  // "fiction/21220/wandering-inn"

  const results: ChapterResult[] = []
  let chapterNum = 1

  $('table#chapters tbody tr, .chapter-row').each((_, el) => {
    const $el = $(el)
    const linkEl = $el.find('a[href*="/chapter/"]').first()
    const href = linkEl.attr('href') ?? ''
    // href = "/fiction/21220/wandering-inn/chapter/1234567/chapter-title"
    if (!href.includes('/chapter/')) return

    const chapterPath = href.slice(1)  // strip leading "/"
    const title = linkEl.text().trim() || null

    results.push({
      source_id: chapterPath,
      number: chapterNum++,
      title,
      chapter_format: 'text',
    })
  })

  // Fallback: chapters may be in a script or different structure
  if (results.length === 0) {
    // Try parsing from the page script (RR sometimes embeds chapter data as JSON)
    const scriptContent = $('script').map((_, s) => $(s).html() ?? '').get().join('\n')
    const jsonMatch = scriptContent.match(/window\.__INITIAL_STATE__\s*=\s*(\{[\s\S]*?\});/)
    if (jsonMatch) {
      try {
        const state = JSON.parse(jsonMatch[1]!)
        const chapterList = state?.fiction?.chapters ?? []
        chapterList.forEach((ch: { id: number; title: string; url?: string }, i: number) => {
          results.push({
            source_id: ch.url ? ch.url.slice(1) : `fiction/${fictionId}/chapter/${ch.id}`,
            number: i + 1,
            title: ch.title || null,
            chapter_format: 'text',
          })
        })
      } catch { /* ignore parse errors */ }
    }
  }

  void fictionPath  // used for source_id construction; keep for clarity
  return results
}

export async function chapterText(chapterPath: string): Promise<string> {
  // chapterPath = "fiction/21220/wandering-inn/chapter/1234567/chapter-title"
  const url = `${BASE}/${chapterPath}`
  const html = await fetchHtml(url)
  const $ = cheerio.load(html)

  const contentEl = $('.chapter-inner.chapter-content, .chapter-content').first()
  if (!contentEl.length) throw new Error(`royalroad: no chapter content found at ${url}`)

  // Remove ads and spoiler toggles before converting
  contentEl.find('.ads-container, .spoiler-toggle, script, style').remove()

  const markdown = td.turndown(contentEl.html() ?? '')
  return markdown
}

export async function meta(fictionId: string): Promise<MetaResult> {
  const res = await fetch(`${BASE}/fiction/${fictionId}`, {
    headers: { 'User-Agent': 'Mozilla/5.0' },
    redirect: 'follow',
  })
  if (!res.ok) throw new Error(`royalroad: fiction ${fictionId} returned ${res.status}`)
  const html = await res.text()
  const $ = cheerio.load(html)

  const description = $('.description.hidden-content').text().trim()
    || $('.synopsis').text().trim()
    || null

  const cover = $('.cover-art img').first().attr('src') ?? null

  const chapterCount = $('table#chapters tbody tr, .chapter-row').length

  const tags = $('.tags .label')
    .map((_, el) => $(el).text().trim())
    .get()
    .filter(Boolean)
    .join(', ') || null

  return {
    description: description || null,
    cover_url: cover,
    chapter_count: chapterCount,
    tags,
  }
}
