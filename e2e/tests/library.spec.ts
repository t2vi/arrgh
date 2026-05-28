import { test, expect } from '../fixtures/auth'

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
})
