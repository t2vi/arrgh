import { test, expect } from '../fixtures/auth'

const BASE = 'http://localhost:8080'

test.describe('Discover', () => {
  test('navigate to non-existent title → shows error state, no crash', async ({ page }) => {
    await page.goto(`${BASE}/title/nonexistent-title-id-00000`)
    await page.waitForTimeout(2000)
    // App should show something — not a blank screen or JS error
    const body = await page.locator('body').textContent()
    expect(body?.trim().length).toBeGreaterThan(0)
    // Page should not crash (no "Something went wrong" unhandled error)
    await expect(page.locator('text=Something went wrong')).toBeHidden()
  })

  test('discover page loads with search input', async ({ page }) => {
    await page.goto(`${BASE}/discover`)
    // Verify the search input is visible — basic smoke test for the Discover UI
    await expect(page.getByPlaceholder(/search for manga/i)).toBeVisible({ timeout: 5_000 })
    await expect(page.getByRole('button', { name: /^search$/i })).toBeVisible()
  })
})
