import { test, expect } from '../fixtures/auth'

const BASE = 'http://localhost:8080'

async function getToken(page: import('@playwright/test').Page): Promise<string> {
  const state = await page.context().storageState()
  const origin = state.origins.find(o => o.origin === BASE)
  return origin?.localStorage.find(item => item.name === 'arrgh_token')?.value ?? ''
}

async function addTitleViaApi(
  page: import('@playwright/test').Page,
  title: string,
  contentType = 'manga',
): Promise<string> {
  const token = await getToken(page)
  const res = await page.request.post(`${BASE}/api/discover/add`, {
    headers: { Authorization: `Bearer ${token}`, 'Content-Type': 'application/json' },
    data: { title, mangaupdates_id: '0', description: null, cover_url: null,
            status: 'ongoing', content_type: contentType, tags: null, author: null, year: null },
  })
  const body = await res.json()
  return body.id as string
}

async function deleteTitleViaApi(page: import('@playwright/test').Page, id: string) {
  const token = await getToken(page)
  await page.request.delete(`${BASE}/api/titles/${id}?delete_files=true`, {
    headers: { Authorization: `Bearer ${token}` },
  })
}

async function waitForSyncDone(page: import('@playwright/test').Page, id: string) {
  const token = await getToken(page)
  const deadline = Date.now() + 30_000
  while (Date.now() < deadline) {
    const res = await page.request.get(`${BASE}/api/titles/${id}`, {
      headers: { Authorization: `Bearer ${token}` },
    })
    const body = await res.json()
    if (body.sync_status === 'ready' || body.sync_status === 'error') return body
    await page.waitForTimeout(1000)
  }
  throw new Error('sync did not complete within 30s')
}

test.describe('Library', () => {
  test('add title → appears in library', async ({ page }) => {
    const id = await addTitleViaApi(page, 'Fixture Manga')
    await waitForSyncDone(page, id)

    await page.goto(`${BASE}/library`)
    await expect(page.getByText('Fixture Manga', { exact: false })).toBeVisible({ timeout: 5_000 })

    await deleteTitleViaApi(page, id)
  })

  test('sync progress overlay visible while building', async ({ page }) => {
    // Add title but DON'T wait for sync — navigate immediately to see overlay
    const id = await addTitleViaApi(page, 'Fixture Manga')

    await page.goto(`${BASE}/library`)
    const card = page.locator('[data-nav]').filter({ hasText: 'Fixture Manga' })
    await expect(card).toBeVisible({ timeout: 5_000 })

    // Wait for sync to finish — overlay should disappear
    await waitForSyncDone(page, id)
    await page.reload()
    await expect(card.locator('text=Building…')).toBeHidden({ timeout: 5_000 })

    await deleteTitleViaApi(page, id)
  })

  test('source match failure → sync warning badge visible', async ({ page }) => {
    const id = await addTitleViaApi(page, 'Fixture No Match')
    await waitForSyncDone(page, id)

    await page.goto(`${BASE}/library`)
    const card = page.locator('[data-nav]').filter({ hasText: 'Fixture No Match' })
    await expect(card).toBeVisible({ timeout: 5_000 })
    // Amber '!' badge appears when has_sync_warnings is true
    await expect(card.locator('[title*="source"]')).toBeVisible({ timeout: 5_000 })

    await deleteTitleViaApi(page, id)
  })

  // ── Chapter sync (ChapterSync module) ─────────────────────────────────────

  test('chapters have has_sources=true after sync', async ({ page }) => {
    const id = await addTitleViaApi(page, 'Fixture Manga')
    await waitForSyncDone(page, id)

    const token = await getToken(page)
    const chapRes = await page.request.get(`${BASE}/api/chapters/title/${id}`, {
      headers: { Authorization: `Bearer ${token}` },
    })
    expect(chapRes.ok()).toBe(true)
    const chapters = await chapRes.json() as { has_sources: boolean; number: number }[]

    expect(chapters.length).toBeGreaterThan(0)
    expect(chapters.every(c => c.has_sources)).toBe(true)

    await deleteTitleViaApi(page, id)
  })

  // ── Manhwa chapters (Bug: fixture didn't handle non-fixture source keys) ──────

  test('manhwa title → chapters rendered after sync', async ({ page }) => {
    // External sources include mangadex (content_types includes "manhwa", priority 10).
    // MatchSourcesAsync calls {fixture}/{source}/search — fixture must handle any source key.
    const id = await addTitleViaApi(page, 'Fixture Manhwa', 'manhwa')
    await waitForSyncDone(page, id)

    const token = await getToken(page)
    const chapRes = await page.request.get(`${BASE}/api/chapters/title/${id}`, {
      headers: { Authorization: `Bearer ${token}` },
    })
    expect(chapRes.ok()).toBe(true)
    const chapters = await chapRes.json() as { has_sources: boolean; number: number }[]

    expect(chapters.length).toBeGreaterThan(0)
    expect(chapters.every(c => c.has_sources)).toBe(true)

    await deleteTitleViaApi(page, id)
  })

  // ── Download status feedback (Bug: "in_progress" vs "downloading" mismatch) ─

  test('chapter row flips to Downloaded after queue completes — no navigation needed', async ({ page }) => {
    // Before fix: DownloaderService set status="in_progress", but frontend polls for
    // status="downloading" — hasActive was always false once download started, polling
    // stopped, and the chapter never flipped without a manual navigate-away-and-back.
    const id = await addTitleViaApi(page, 'Fixture Manga')
    await waitForSyncDone(page, id)

    const token = await getToken(page)
    const chapters = await (await page.request.get(`${BASE}/api/chapters/title/${id}`, {
      headers: { Authorization: `Bearer ${token}` },
    })).json() as { id: string; number: number }[]
    const ch1 = chapters.find(c => c.number === 1)!

    await page.goto(`${BASE}/titles/${id}`)

    // Expand all chapters so ch1 row is visible
    const showAll = page.getByRole('button', { name: /view all/i })
    if (await showAll.isVisible()) await showAll.click()

    // Click download on chapter 1
    const chapterRow = page.locator('[data-chapter-id]').filter({ has: page.locator('text=Chapter 1') }).first()
    const downloadBtn = chapterRow.locator('button[title="Download & read"]')
    await expect(downloadBtn).toBeVisible({ timeout: 5_000 })
    await downloadBtn.click()

    // Chapter row must flip to "Downloaded" (BookOpen icon) WITHOUT navigating away
    const bookOpenIcon = chapterRow.locator('[data-lucide="book-open"], svg[data-icon="book-open"]')
    await expect(bookOpenIcon).toBeVisible({ timeout: 15_000 })

    await deleteTitleViaApi(page, id)
  })

  test('queue row shows "downloading" badge — not "in_progress" — during active download', async ({ page }) => {
    // Before fix: backend sent status="in_progress" which the QueueRow STATUS map doesn't
    // know about, falling back to the clock/pending style and showing "In_progress" as badge text.
    const id = await addTitleViaApi(page, 'Fixture Manga')
    await waitForSyncDone(page, id)

    const token = await getToken(page)
    const chapters = await (await page.request.get(`${BASE}/api/chapters/title/${id}`, {
      headers: { Authorization: `Bearer ${token}` },
    })).json() as { id: string; number: number }[]
    const ch1 = chapters.find(c => c.number === 1)!

    // Queue the download via API
    await page.request.post(`${BASE}/api/chapters/${ch1.id}/download`, {
      headers: { Authorization: `Bearer ${token}` },
    })

    // Navigate to queue page immediately — download may still be in-progress
    await page.goto(`${BASE}/queue`)

    // The badge text must never say "In_progress" — only "downloading", "done", "pending", "error"
    const inProgressText = page.locator('text=In_progress, text=in_progress')
    // Wait a tick for render, then assert absent
    await page.waitForTimeout(500)
    await expect(inProgressText).toHaveCount(0)

    await deleteTitleViaApi(page, id)
  })

  test('manual sync is idempotent — chapter count unchanged on re-sync', async ({ page }) => {
    const id = await addTitleViaApi(page, 'Fixture Manga')
    await waitForSyncDone(page, id)

    const token = await getToken(page)
    const before = await (await page.request.get(`${BASE}/api/chapters/title/${id}`, {
      headers: { Authorization: `Bearer ${token}` },
    })).json() as unknown[]

    // Trigger manual sync
    await page.request.post(`${BASE}/api/titles/${id}/sync`, {
      headers: { Authorization: `Bearer ${token}` },
    })
    await waitForSyncDone(page, id)

    const after = await (await page.request.get(`${BASE}/api/chapters/title/${id}`, {
      headers: { Authorization: `Bearer ${token}` },
    })).json() as unknown[]

    expect(after.length).toBe(before.length)

    await deleteTitleViaApi(page, id)
  })
})
