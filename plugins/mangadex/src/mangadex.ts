const BASE = 'https://api.mangadex.org'
const UA = 'arrgh-mangadex-plugin/1.0 (https://github.com/t2vi/arrgh)'

// ── Wire types ────────────────────────────────────────────────────────────────

interface Relationship {
  type: string
  id: string
  attributes?: Record<string, unknown>
}

interface MangaAttributes {
  title: Record<string, string>
  description: Record<string, string>
  status: string
  year: number | null
  originalLanguage: string
  contentRating: string
  tags: Array<{ attributes: { name: Record<string, string> } }>
}

interface MangaEntity {
  id: string
  attributes: MangaAttributes
  relationships: Relationship[]
}

interface ChapterAttributes {
  volume: string | null
  chapter: string | null
  title: string | null
  translatedLanguage: string
  externalUrl: string | null
}

interface ChapterEntity {
  id: string
  attributes: ChapterAttributes
}

interface AtHomeResponse {
  baseUrl: string
  chapter: {
    hash: string
    data: string[]
    dataSaver: string[]
  }
}

interface PagedResponse<T> {
  data: T[]
  total: number
  limit: number
  offset: number
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

export interface MetaResponse {
  description: string | null
  cover_url: string | null
  chapter_count: number
}

// ── Client ────────────────────────────────────────────────────────────────────

async function fetchMD<T>(path: string, params?: Record<string, string | string[]>): Promise<T> {
  const url = new URL(BASE + path)
  if (params) {
    for (const [k, v] of Object.entries(params)) {
      if (Array.isArray(v)) {
        v.forEach((item) => url.searchParams.append(k, item))
      } else {
        url.searchParams.set(k, v)
      }
    }
  }
  const res = await fetch(url, { headers: { 'User-Agent': UA } })
  if (!res.ok) throw new Error(`MangaDex ${res.status} for ${path}`)
  return res.json() as Promise<T>
}

function coverUrl(manga: MangaEntity): string | null {
  const rel = manga.relationships.find((r) => r.type === 'cover_art')
  const fileName = rel?.attributes?.['fileName'] as string | undefined
  if (!fileName) return null
  return `https://uploads.mangadex.org/covers/${manga.id}/${fileName}.256.jpg`
}

function authorName(manga: MangaEntity): string | null {
  const rel = manga.relationships.find((r) => r.type === 'author')
  return (rel?.attributes?.['name'] as string | undefined) ?? null
}

function bestTitle(titles: Record<string, string>): string {
  return titles['en'] ?? titles['ja-ro'] ?? titles['ja'] ?? Object.values(titles)[0] ?? 'Unknown'
}

function toSearchItem(m: MangaEntity): SearchItem {
  const tags = m.attributes.tags
    .map((t) => t.attributes.name['en'] ?? '')
    .filter(Boolean)
    .join(', ')

  return {
    id: m.id,
    title: bestTitle(m.attributes.title),
    description: m.attributes.description['en'] ?? null,
    cover_url: coverUrl(m),
    status: m.attributes.status,
    author: authorName(m),
    year: m.attributes.year,
    tags: tags || null,
  }
}

const INCLUDES = ['cover_art', 'author']

export async function search(query: string): Promise<SearchItem[]> {
  const data = await fetchMD<PagedResponse<MangaEntity>>('/manga', {
    title: query,
    limit: '20',
    'includes[]': INCLUDES,
    'order[relevance]': 'desc',
  })
  return data.data.map(toSearchItem)
}

export async function trending(): Promise<SearchItem[]> {
  const data = await fetchMD<PagedResponse<MangaEntity>>('/manga', {
    limit: '20',
    'includes[]': INCLUDES,
    'order[followedCount]': 'desc',
    'contentRating[]': ['safe', 'suggestive'],
  })
  return data.data.map(toSearchItem)
}

export async function meta(mangaId: string, langs: string[]): Promise<MetaResponse> {
  const [mangaData, feedData] = await Promise.all([
    fetchMD<{ data: MangaEntity }>(`/manga/${mangaId}`, { 'includes[]': INCLUDES }),
    fetchMD<PagedResponse<ChapterEntity>>(`/manga/${mangaId}/feed`, {
      limit: '1',
      'translatedLanguage[]': langs,
      'order[chapter]': 'desc',
    }),
  ])

  const manga = mangaData.data
  const total = feedData.total

  return {
    description: manga.attributes.description['en'] ?? null,
    cover_url: coverUrl(manga),
    chapter_count: total,
  }
}

export async function chapters(mangaId: string, langs: string[]): Promise<ChapterItem[]> {
  const PAGE = 500
  let offset = 0
  const all: ChapterItem[] = []

  // eslint-disable-next-line no-constant-condition
  while (true) {
    const data = await fetchMD<PagedResponse<ChapterEntity>>(`/manga/${mangaId}/feed`, {
      limit: String(PAGE),
      offset: String(offset),
      'translatedLanguage[]': langs,
      'order[chapter]': 'asc',
      'order[volume]': 'asc',
      'contentRating[]': ['safe', 'suggestive', 'erotica', 'pornographic'],
    })

    for (const ch of data.data) {
      if (ch.attributes.externalUrl) continue  // hosted externally, no pages via MangaDex
      const num = parseFloat(ch.attributes.chapter ?? '')
      if (isNaN(num)) continue
      all.push({
        source_id: ch.id,
        number: num,
        volume: ch.attributes.volume ? parseFloat(ch.attributes.volume) : null,
        title: ch.attributes.title ?? null,
      })
    }

    offset += data.data.length
    if (offset >= data.total) break
  }

  // Deduplicate: keep one entry per chapter number (prefer lowest source_id for stability)
  const seen = new Map<number, ChapterItem>()
  for (const ch of all) {
    if (!seen.has(ch.number)) seen.set(ch.number, ch)
  }

  return Array.from(seen.values())
}

export async function pages(chapterId: string): Promise<string[]> {
  const data = await fetchMD<AtHomeResponse>(`/at-home/server/${chapterId}`)
  const { baseUrl, chapter } = data
  return chapter.data.map((f) => `${baseUrl}/data/${chapter.hash}/${f}`)
}
