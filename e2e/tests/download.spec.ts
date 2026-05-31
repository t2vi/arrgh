import { test, expect } from '../fixtures/auth'
import { allure } from 'allure-playwright'

test.beforeEach(async () => {
  await allure.epic('Queue')
  await allure.feature('Download')
})


const BASE = 'http://localhost:8080'

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

test.describe('Download', () => {
  test('queue chapter → download reaches done', async ({ page }) => {
    const id = await addTitleViaApi(page, 'Fixture Manga')
    await waitForSyncDone(page, id)

    await page.goto(`${BASE}/title/${id}`)
    // Wait for chapters to appear (fixture returns 3 chapters)
    await expect(page.locator('[title="Download & read"]').first()).toBeVisible({ timeout: 10_000 })
    await page.locator('[title="Download & read"]').first().click()

    // Queue item should eventually reach done
    await page.goto(`${BASE}/queue`)
    await expect(page.getByText(/Fixture Manga/i).first()).toBeVisible({ timeout: 10_000 })

    // Navigate back to title — chapter should be marked downloaded
    await page.goto(`${BASE}/title/${id}`)
    await expect(page.locator('[title="Read"]').first()).toBeVisible({ timeout: 30_000 })

    await deleteTitleViaApi(page, id)
  })

  test('source 502 on chapter sync → sync warning shown', async ({ page }) => {
    // Fixture 502: chapters endpoint returns 502 → Err → sync_warning set → amber badge visible
    const id = await addTitleViaApi(page, 'Fixture 502')
    const titleData = await waitForSyncDone(page, id)

    // Title found in source (any_ok=true), but chapter sync failed → warning, not error status
    expect(titleData.sync_status).toBe('ready')
    expect(titleData.has_sync_warnings).toBe(true)

    // Verify amber badge visible in library UI
    await page.goto(`${BASE}/library`)
    const card = page.locator('[data-nav]').filter({ hasText: 'Fixture 502' })
    await expect(card).toBeVisible({ timeout: 5_000 })
    await expect(card.locator('[title*="source"]')).toBeVisible({ timeout: 5_000 })

    await deleteTitleViaApi(page, id)
  })

  test('empty pages → queue item shows error', async ({ page }) => {
    const id = await addTitleViaApi(page, 'Fixture Empty Pages')
    await waitForSyncDone(page, id)

    await page.goto(`${BASE}/title/${id}`)
    await expect(page.locator('[title="Download & read"]').first()).toBeVisible({ timeout: 10_000 })
    await page.locator('[title="Download & read"]').first().click()

    // Queue item should reach error status (0 pages guard)
    await page.goto(`${BASE}/queue`)
    await expect(page.getByText(/error/i).first()).toBeVisible({ timeout: 30_000 })

    await deleteTitleViaApi(page, id)
  })
})
