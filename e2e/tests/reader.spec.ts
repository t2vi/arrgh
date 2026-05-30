import { test, expect } from '../fixtures/auth'

const BASE = 'http://localhost:8080'

// ── Helpers ───────────────────────────────────────────────────────────────────

async function getToken(page: import('@playwright/test').Page): Promise<string> {
  const state = await page.context().storageState()
  const origin = state.origins.find(o => o.origin === BASE)
  return origin?.localStorage.find(item => item.name === 'arrgh_token')?.value ?? ''
}

async function addTitleViaApi(page: import('@playwright/test').Page, title: string): Promise<string> {
  const token = await getToken(page)
  const res = await page.request.post(`${BASE}/api/discover/add`, {
    headers: { Authorization: `Bearer ${token}`, 'Content-Type': 'application/json' },
    data: { title, mangaupdates_id: '0', description: null, cover_url: null,
            status: 'ongoing', content_type: 'manga', tags: null, author: null, year: null },
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

async function getFirstChapterId(page: import('@playwright/test').Page, titleId: string): Promise<string> {
  const token = await getToken(page)
  const res = await page.request.get(`${BASE}/api/chapters/title/${titleId}`, {
    headers: { Authorization: `Bearer ${token}` },
  })
  const chapters = await res.json() as { id: string; number: number }[]
  chapters.sort((a, b) => a.number - b.number)
  return chapters[0].id
}

async function downloadAndOpenReader(page: import('@playwright/test').Page, titleId: string): Promise<string> {
  const chapterId = await getFirstChapterId(page, titleId)

  // Queue download via API
  const token = await getToken(page)
  await page.request.post(`${BASE}/api/chapters/${chapterId}/download`, {
    headers: { Authorization: `Bearer ${token}` },
  })

  // Wait for download to complete — poll chapter until downloaded=true
  const deadline = Date.now() + 30_000
  while (Date.now() < deadline) {
    const res = await page.request.get(`${BASE}/api/chapters/title/${titleId}`, {
      headers: { Authorization: `Bearer ${token}` },
    })
    const chapters = await res.json() as { id: string; downloaded: boolean }[]
    if (chapters.find(c => c.id === chapterId)?.downloaded) break
    await page.waitForTimeout(1000)
  }

  await page.goto(`${BASE}/reader/${chapterId}`)
  return chapterId
}

// ── Tests ─────────────────────────────────────────────────────────────────────

test.describe('Reader — zoom control', () => {
  let titleId: string

  test.beforeEach(async ({ page }) => {
    titleId = await addTitleViaApi(page, 'Fixture Manga')
    await waitForSyncDone(page, titleId)
  })

  test.afterEach(async ({ page }) => {
    if (titleId) await deleteTitleViaApi(page, titleId)
  })

  test('zoom button visible for manga chapter', async ({ page }) => {
    await downloadAndOpenReader(page, titleId)
    await expect(page.getByTitle('Zoom')).toBeVisible({ timeout: 10_000 })
  })

  test('zoom dropdown shows all 5 levels', async ({ page }) => {
    await downloadAndOpenReader(page, titleId)
    await page.getByTitle('Zoom').click()

    for (const level of ['50%', '75%', '100%', '125%', '150%']) {
      await expect(page.getByRole('button', { name: level })).toBeVisible()
    }
  })

  test('selecting 150% sets image max-width to 1200px', async ({ page }) => {
    await downloadAndOpenReader(page, titleId)
    await page.getByTitle('Zoom').click()
    await page.getByRole('button', { name: '150%' }).click()

    // Dropdown closes and image/container gets maxWidth = 150 * 8 = 1200px
    await expect(page.getByRole('button', { name: '50%' })).toBeHidden()

    const img = page.locator('img[src*="/api/media"]').first()
    await expect(img).toBeVisible({ timeout: 5_000 })
    const style = await img.getAttribute('style') ?? ''
    expect(style).toContain('max-width: 1200px')
  })

  test('selecting 50% sets image max-width to 400px', async ({ page }) => {
    await downloadAndOpenReader(page, titleId)
    await page.getByTitle('Zoom').click()
    await page.getByRole('button', { name: '50%' }).click()

    const img = page.locator('img[src*="/api/media"]').first()
    await expect(img).toBeVisible({ timeout: 5_000 })
    const style = await img.getAttribute('style') ?? ''
    expect(style).toContain('max-width: 400px')
  })

  test('zoom level persists via localStorage after reload', async ({ page }) => {
    await downloadAndOpenReader(page, titleId)
    await page.getByTitle('Zoom').click()
    await page.getByRole('button', { name: '125%' }).click()

    // Reload — zoom should be restored from localStorage
    await page.reload()
    await expect(page.getByTitle('Zoom')).toBeVisible({ timeout: 10_000 })

    // Open zoom picker and verify 125% is highlighted (active)
    await page.getByTitle('Zoom').click()
    const btn125 = page.getByRole('button', { name: '125%' })
    await expect(btn125).toHaveClass(/bg-primary/)
  })

})

test.describe('Reader — scroll mode zoom', () => {
  let titleId: string

  test.beforeEach(async ({ page }) => {
    titleId = await addTitleViaApi(page, 'Fixture Manga')
    await waitForSyncDone(page, titleId)
  })

  test.afterEach(async ({ page }) => {
    if (titleId) await deleteTitleViaApi(page, titleId)
  })

  test('zoom applies max-width in scroll reader', async ({ page }) => {
    await downloadAndOpenReader(page, titleId)

    // Switch to scroll mode
    await page.getByTitle('Switch to scroll').click()

    // Apply 75% zoom
    await page.getByTitle('Zoom').click()
    await page.getByRole('button', { name: '75%' }).click()

    // Scroll reader renders a column container with maxWidth = 75 * 8 = 600px
    // Locate it by its inline style (ScrollReader inner flex div)
    const container = page.locator('[style*="max-width: 600px"]').first()
    await expect(container).toBeVisible({ timeout: 5_000 })
  })
})
