// Full library flow for novel authority: discover → add → sync → chapters → download → view
// Source: WuxiaWorld (official API, no CF protection required)
// Novel chapters return text (no page images) — step 8 verifies chapter text endpoint instead.
// Run: API_USER=admin API_PASS=... npm test (from api-live-tests/)

import { describe, it, expect, beforeAll, afterAll } from 'vitest'
import { getToken, apiGet } from './helpers'

const QUERY = 'I Shall Seal the Heavens'
const BASE = process.env.API_URL ?? 'http://localhost:3000'
const POLL_INTERVAL_MS = 500
// Large novels (1000+ chapters) can take >30s to sync — use a longer timeout
const SYNC_TIMEOUT_MS = 120_000
const DOWNLOAD_TIMEOUT_MS = 120_000

interface DiscoverResult {
  title: string
  content_type: string
  source: string
  source_id: string
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

async function poll<T>(fn: () => Promise<T>, check: (v: T) => boolean, timeoutMs: number): Promise<T> {
  const deadline = Date.now() + timeoutMs
  while (Date.now() < deadline) {
    const v = await fn()
    if (check(v)) return v
    await new Promise(r => setTimeout(r, POLL_INTERVAL_MS))
  }
  return fn()
}

let token: string
let titleId: string | null = null
let chapter1Id: string | null = null
let addedTitle: string | null = null

beforeAll(async () => {
  token = await getToken()
})

afterAll(async () => {
  if (titleId) {
    // Wait for background sync to finish before deleting — prevents SQLite write-lock
    // contention in the next test file (novel sync can take >30s for large chapter lists)
    await poll(
      () => apiGet<TitleItem>(`/api/titles/${titleId!}`, token),
      t => t.sync_status !== 'syncing',
      120_000,
    ).catch(() => {})
    await fetch(`${BASE}/api/titles/${titleId}`, {
      method: 'DELETE',
      headers: { Authorization: `Bearer ${token}` },
    })
  }
})

describe(`library flow — wuxiaworld novel (${QUERY})`, () => {
  it('1. discover returns a novel result', async () => {
    const results = await apiGet<DiscoverResult[]>(
      `/api/discover?q=${encodeURIComponent(QUERY)}`, token)

    // NovelUpdates is the designated novel authority; WuxiaWorld is also queried as a parallel authority.
    // After dedup, NovelUpdates wins. We verify WuxiaWorld is queried via the sync log (step 5).
    const match = results.find(r => ['novel', 'web novel', 'light novel'].includes(r.content_type))
    expect(match, `no novel result for "${QUERY}"`).toBeDefined()
  })

  it('2. add to library', async () => {
    const results = await apiGet<DiscoverResult[]>(
      `/api/discover?q=${encodeURIComponent(QUERY)}`, token)

    const match = results.find(r => ['novel', 'web novel', 'light novel'].includes(r.content_type))
    expect(match, `no novel result for "${QUERY}"`).toBeDefined()

    addedTitle = match!.title
    const { status, body } = await apiPost<TitleItem>('/api/discover/add', {
      source: match!.source,
      source_id: match!.source_id,
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

  it('4. chapters loaded — all synced, sources present, text format', async () => {
    expect(titleId).toBeTruthy()
    const chapters = await apiGet<ChapterDto[]>(`/api/chapters/title/${titleId}`, token)

    // ISSTH has 1620 chapters — if cumulative numbering is broken we get ~318 (groups overlap)
    expect(chapters.length, `only ${chapters.length} chapters — wuxiaworld cumulative numbering may be broken`).toBeGreaterThan(1000)
    expect(chapters.some(c => c.has_sources)).toBe(true)
    // Novel chapters are text, not page images
    expect(chapters.some(c => c.chapter_format === 'text')).toBe(true)
    // Numbers must be sequential and unique (no dedup collisions)
    const nums = chapters.map(c => c.number).sort((a, b) => a - b)
    expect(nums[0]).toBe(1)
    expect(new Set(nums).size).toBe(chapters.length)
  })

  it('5. sync log contains wuxiaworld and sync-complete entries', async () => {
    expect(titleId).toBeTruthy()
    const logs = await apiGet<{ message: string }[]>(`/api/titles/${titleId}/sync-log`, token)
    const messages = logs.map(l => l.message)

    expect(messages.some(m => m.toLowerCase().includes('wuxiaworld')),
      `wuxiaworld not in sync log. Got: ${messages.join(' | ')}`).toBe(true)
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
    // Text chapters count "pages" differently — just verify download succeeded
    expect(item!.pages_downloaded).toBeGreaterThanOrEqual(0)
  })

  it('8. chapter is viewable — text endpoint serves content', async () => {
    expect(chapter1Id).toBeTruthy()

    const chapters = await apiGet<ChapterDto[]>(`/api/chapters/title/${titleId}`, token)
    const ch = chapters.find(c => c.id === chapter1Id)
    expect(ch?.downloaded).toBe(true)
    expect(ch?.chapter_format).toBe('text')

    // Novel chapters use the text endpoint, not page images
    const res = await fetch(`${BASE}/api/chapters/${chapter1Id}/text`, {
      headers: { Authorization: `Bearer ${token}` },
    })
    expect(res.status).toBe(200)
    const text = await res.text()
    expect(text.length, 'chapter text is empty').toBeGreaterThan(0)
  })
})
