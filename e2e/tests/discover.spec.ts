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

test.describe('Discover', () => {
  test('navigate to non-existent title → shows error state, no crash', async ({ page }) => {
    await page.goto(`${BASE}/title/nonexistent-title-id-00000`)
    await page.waitForTimeout(2000)
    const body = await page.locator('body').textContent()
    expect(body?.trim().length).toBeGreaterThan(0)
    await expect(page.locator('text=Something went wrong')).toBeHidden()
  })

  test('discover page loads with search input', async ({ page }) => {
    await page.goto(`${BASE}/discover`)
    await expect(page.getByPlaceholder(/search for manga/i)).toBeVisible({ timeout: 5_000 })
    await expect(page.getByRole('button', { name: /^search$/i })).toBeVisible()
  })

  // ── ContentTypeFilter (ADR 0031) ───────────────────────────────────────────

  test('content type filter pills appear after search', async ({ page }) => {
    await page.goto(`${BASE}/discover`)
    await page.getByPlaceholder(/search for manga/i).fill('Fixture Manga')
    await page.getByRole('button', { name: /^search$/i }).click()

    // Filter pills only appear once results are loaded
    await expect(page.getByRole('button', { name: 'All' })).toBeVisible({ timeout: 15_000 })
    // Fixture plugin returns manga results
    await expect(page.getByRole('button', { name: 'Manga' })).toBeVisible()
  })

  test('All pill active by default, Manga pill activates on click', async ({ page }) => {
    await page.goto(`${BASE}/discover`)
    await page.getByPlaceholder(/search for manga/i).fill('Fixture Manga')
    await page.getByRole('button', { name: /^search$/i }).click()

    await expect(page.getByRole('button', { name: 'All' })).toBeVisible({ timeout: 15_000 })

    // "All" starts active (no content_type filter applied)
    const allBtn = page.getByRole('button', { name: 'All' })
    await expect(allBtn).toHaveClass(/bg-primary/)

    // Click "Manga" → it becomes active
    await page.getByRole('button', { name: 'Manga' }).click()
    const mangaBtn = page.getByRole('button', { name: 'Manga' })
    await expect(mangaBtn).toHaveClass(/bg-primary/)
    await expect(allBtn).not.toHaveClass(/bg-primary/)

    // Click "All" → back to no filter
    await allBtn.click()
    await expect(allBtn).toHaveClass(/bg-primary/)
  })

  test('adding from discover search lands in library', async ({ page }) => {
    await page.goto(`${BASE}/discover`)
    await page.getByPlaceholder(/search for manga/i).fill('Fixture Add Test')
    await page.getByRole('button', { name: /^search$/i }).click()

    // Wait for result row with Add button
    const addBtn = page.getByRole('button', { name: 'Add' }).first()
    await expect(addBtn).toBeVisible({ timeout: 15_000 })
    await addBtn.click()

    // Button changes to "In Library" after add
    await expect(page.getByRole('button', { name: 'In Library' }).first()).toBeVisible({ timeout: 10_000 })

    // Clean up — find the title id via API and delete
    const token = await getToken(page)
    const list = await page.request.get(`${BASE}/api/titles?search=Fixture+Add+Test`, {
      headers: { Authorization: `Bearer ${token}` },
    })
    const { items } = await list.json() as { items: { id: string }[] }
    if (items[0]) await deleteTitleViaApi(page, items[0].id)
  })
})
