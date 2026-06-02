// Full library flow: discover → add → sync → chapters → download → view
// Requires: API server (API_URL, default localhost:3000) + plugin-host (port 4000)
// Run: API_USER=admin API_PASS=... npm test (from api-live-tests/)

import { describe, it, expect, beforeAll, afterAll } from 'vitest'
import { getToken, apiGet } from './helpers'

const QUERY = 'KayaNetori'
const EXPECTED_TITLE = 'KayaNetori Kaya-Nee Series Aizou Ban'
const BASE = process.env.API_URL ?? 'http://localhost:3000'
const POLL_INTERVAL_MS = 500
const SYNC_TIMEOUT_MS = 30_000
const DOWNLOAD_TIMEOUT_MS = 120_000

interface DiscoverResult {
  title: string
  content_type: string
  source: string
  mangaupdates_id: string
  is_explicit: boolean
}

interface TitleItem {
  id: string
  sync_status: string
  total_chapters: number
}

interface ChapterDto {
  id: string
  number: number
  downloaded: boolean
  has_sources: boolean
  page_count: number
  chapter_format: string
}

interface QueueItemDto {
  chapter_id: string
  status: string
  pages_downloaded: number
  pages_total: number
}

async function apiPost<T>(path: string, body: unknown, tok: string): Promise<{ status: number; body: T }> {
  const res = await fetch(`${BASE}${path}`, {
    method: 'POST',
    headers: { Authorization: `Bearer ${tok}`, 'Content-Type': 'application/json' },
    body: JSON.stringify(body),
  })
  const text = await res.text()
  const parsed = text ? JSON.parse(text) : null
  return { status: res.status, body: parsed }
}

async function poll<T>(
  fn: () => Promise<T>,
  check: (v: T) => boolean,
  timeoutMs: number,
): Promise<T> {
  const deadline = Date.now() + timeoutMs
  while (Date.now() < deadline) {
    const v = await fn()
    if (check(v)) return v
    await new Promise(r => setTimeout(r, POLL_INTERVAL_MS))
  }
  const final = await fn()
  return final
}

let token: string
let titleId: string | null = null
let chapter1Id: string | null = null

beforeAll(async () => {
  token = await getToken()
})

afterAll(async () => {
  if (titleId) {
    await fetch(`${BASE}/api/titles/${titleId}`, {
      method: 'DELETE',
      headers: { Authorization: `Bearer ${token}` },
    })
  }
})

describe(`library flow — ${EXPECTED_TITLE}`, () => {
  it('1. discover returns the title', async () => {
    const results = await apiGet<DiscoverResult[]>(
      `/api/discover?q=${encodeURIComponent(QUERY)}`, token)

    const match = results.find(r => r.title === EXPECTED_TITLE)
    expect(match, `"${EXPECTED_TITLE}" not found in discover results`).toBeDefined()
  })

  it('2. add to library — nhentai result preferred', async () => {
    const results = await apiGet<DiscoverResult[]>(
      `/api/discover?q=${encodeURIComponent(QUERY)}`, token)

    // Prefer hentai content_type result (nhentai authority) so source matching routes correctly.
    // nhentai returns query as the title, not the full series title — match by content_type first.
    // Falls back to MU result if nhentai isn't available (plugin-host/CloakBrowser down).
    const match =
      results.find(r => r.content_type === 'hentai') ??
      results.find(r => r.title === EXPECTED_TITLE)
    expect(match, `"${EXPECTED_TITLE}" not found`).toBeDefined()

    const { status, body } = await apiPost<TitleItem>('/api/discover/add', {
      mangaupdates_id: match!.mangaupdates_id,
      source: match!.source,
      source_id: match!.mangaupdates_id,
      title: match!.title,
      content_type: match!.content_type,
      status: 'complete',
      is_explicit: match!.is_explicit,
    }, token)

    expect(status).toBe(200)
    titleId = body.id
    expect(titleId).toBeTruthy()
  })

  it('3. sync finishes (sync_status → ready)', async () => {
    expect(titleId, 'no titleId — step 2 failed').toBeTruthy()

    const title = await poll(
      () => apiGet<TitleItem>(`/api/titles/${titleId}`, token),
      t => t.sync_status === 'ready',
      SYNC_TIMEOUT_MS,
    )
    expect(title.sync_status).toBe('ready')
  })

  it('4. chapters loaded with sources', async () => {
    expect(titleId).toBeTruthy()
    const chapters = await apiGet<ChapterDto[]>(`/api/chapters/title/${titleId}`, token)

    expect(chapters.length, 'no chapters — nhentai source matching may have failed (plugin-host down?)').toBeGreaterThan(0)
    expect(chapters.some(c => c.has_sources)).toBe(true)
  })

  it('5. sync log contains nhentai and sync-complete entries', async () => {
    expect(titleId).toBeTruthy()
    const logs = await apiGet<{ message: string }[]>(`/api/titles/${titleId}/sync-log`, token)
    const messages = logs.map(l => l.message)

    expect(messages.some(m => m.toLowerCase().includes('nhentai')),
      `nhentai not in sync log. Got: ${messages.join(' | ')}`).toBe(true)
    expect(messages.some(m => m.toLowerCase().includes('sync complete'))).toBe(true)
  })

  it('6. queue chapter 1 for download', async () => {
    expect(titleId).toBeTruthy()
    const chapters = await apiGet<ChapterDto[]>(`/api/chapters/title/${titleId}`, token)

    const ch = chapters
      .filter(c => c.has_sources && !c.downloaded)
      .sort((a, b) => a.number - b.number)[0]
    expect(ch, 'no downloadable chapter found').toBeDefined()

    chapter1Id = ch!.id
    const { status } = await apiPost<unknown>(`/api/chapters/${chapter1Id}/download`, {}, token)
    expect(status).toBe(202)
  })

  it('7. download completes', async () => {
    expect(titleId).toBeTruthy()
    expect(chapter1Id).toBeTruthy()

    const queue = await poll(
      () => apiGet<QueueItemDto[]>(`/api/queue/title/${titleId}`, token),
      items => items.some(i => i.chapter_id === chapter1Id && i.status === 'done'),
      DOWNLOAD_TIMEOUT_MS,
    )

    const item = queue.find(i => i.chapter_id === chapter1Id)
    expect(item?.status, `download status: ${item?.status}`).toBe('done')
    expect(item!.pages_downloaded).toBeGreaterThan(0)
  })

  it('8. chapter is viewable — page 0 serves a 200', async () => {
    expect(chapter1Id).toBeTruthy()

    // Verify chapter is downloaded with pages
    const chapters = await apiGet<ChapterDto[]>(`/api/chapters/title/${titleId}`, token)
    const ch = chapters.find(c => c.id === chapter1Id)
    expect(ch?.downloaded).toBe(true)
    expect(ch?.page_count, 'page_count is 0 after download').toBeGreaterThan(0)

    // Verify page 0 is served
    const res = await fetch(`${BASE}/api/media/page/${chapter1Id}/0`, {
      headers: { Authorization: `Bearer ${token}` },
    })
    expect(res.status).toBe(200)
    expect(res.headers.get('content-type')).toMatch(/^image\//)
  })
})
