import { test, expect } from '../fixtures/auth'

const BASE = 'http://localhost:8080'

// Mock all 4 trending lanes so tests are deterministic regardless of MU/AniList availability
async function mockTrendingLanes(page: import('@playwright/test').Page) {
  const mangaItem = {
    mangaupdates_id: 'e2e-1', source: 'mangaupdates', title: 'E2E Manga',
    description: null, cover_url: null, status: 'ongoing', author: null,
    year: null, tags: null, content_type: 'manga', in_library: false,
    library_id: null, is_explicit: false,
  }
  const manhwaItem = { ...mangaItem, mangaupdates_id: 'e2e-2', title: 'E2E Manhwa', content_type: 'manhwa', source: 'anilist' }
  const manhuaItem = { ...mangaItem, mangaupdates_id: 'e2e-3', title: 'E2E Manhua', content_type: 'manhua', source: 'anilist' }

  await page.route("**/api/discover/trending/manga", r => r.fulfill({ json: [mangaItem, { ...mangaItem, mangaupdates_id: 'e2e-1b', title: 'E2E Manga 2' }] }))
  await page.route("**/api/discover/trending/manhwa", r => r.fulfill({ json: [manhwaItem, { ...manhwaItem, mangaupdates_id: 'e2e-2b', title: 'E2E Manhwa 2' }] }))
  await page.route("**/api/discover/trending/manhua", r => r.fulfill({ json: [manhuaItem, { ...manhuaItem, mangaupdates_id: 'e2e-3b', title: 'E2E Manhua 2' }] }))
  await page.route("**/api/discover/trending/adult-manhwa", r => r.fulfill({ status: 403, json: { error: 'forbidden' } }))
}

test('home page shows 4 trending lane headings', async ({ page }) => {
  await mockTrendingLanes(page)
  await page.goto(`${BASE}/`)
  await expect(page.getByRole('heading', { name: /trending manga/i })).toBeVisible({ timeout: 8000 })
  await expect(page.getByRole('heading', { name: /trending manhwa/i })).toBeVisible()
  await expect(page.getByRole('heading', { name: /trending manhua/i })).toBeVisible()
})

test('each trending lane shows cards from its source', async ({ page }) => {
  await mockTrendingLanes(page)
  await page.goto(`${BASE}/`)
  await expect(page.getByText('E2E Manga', { exact: true })).toBeVisible({ timeout: 8000 })
  await expect(page.getByText('E2E Manhwa', { exact: true })).toBeVisible()
  await expect(page.getByText('E2E Manhua', { exact: true })).toBeVisible()
})

test('adult manhwa lane hidden when user lacks explicit permission', async ({ page }) => {
  await mockTrendingLanes(page)
  await page.goto(`${BASE}/`)
  // Non-explicit user (fixture user) — adult lane must not render
  await page.waitForTimeout(2000) // let all lanes settle
  await expect(page.getByRole('heading', { name: /18\+/i })).not.toBeVisible()
})

test('clicking a trending card opens the modal', async ({ page }) => {
  await mockTrendingLanes(page)
  await page.goto(`${BASE}/`)
  await page.getByText('E2E Manga', { exact: true }).click()
  await expect(page.getByRole('dialog')).toBeVisible({ timeout: 4000 })
})
