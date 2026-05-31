import { test, expect } from '@playwright/test'
import { allure } from 'allure-playwright'

test.beforeEach(async () => {
  await allure.epic('Auth')
  await allure.feature('Login')
})


const BASE = 'http://localhost:8080'
const ADMIN = { username: 'test-admin', password: 'testpassword123' }

test.describe('Auth', () => {
  test.use({ storageState: { cookies: [], origins: [] } }) // no pre-auth

  test('unauthenticated user is redirected to login', async ({ page }) => {
    await page.goto(`${BASE}/library`)
    await expect(page).toHaveURL(/login|setup/, { timeout: 5_000 })
  })

  test('login → navigate → logout → redirected to login', async ({ page }) => {
    await page.goto(`${BASE}/login`)
    await page.getByPlaceholder('Username').fill(ADMIN.username)
    await page.getByPlaceholder('••••••••').fill(ADMIN.password)
    await page.getByRole('button', { name: /sign in/i }).click()

    await expect(page).not.toHaveURL(/login/, { timeout: 10_000 })
    // Wait for the sidebar to confirm auth state is persisted to localStorage
    await expect(page.getByRole('navigation')).toBeVisible({ timeout: 5_000 })

    // Logout is in settings
    await page.goto(`${BASE}/settings`)
    await page.getByRole('button', { name: /logout/i }).click()
    await expect(page).toHaveURL(/login/, { timeout: 5_000 })
  })

  test('wrong password shows error', async ({ page }) => {
    await page.goto(`${BASE}/login`)
    await page.getByPlaceholder('Username').fill(ADMIN.username)
    await page.getByPlaceholder('••••••••').fill('wrongpassword')
    await page.getByRole('button', { name: /sign in/i }).click()
    await expect(page).toHaveURL(/login/)
    await expect(page.locator('.text-destructive')).toBeVisible({ timeout: 5_000 })
  })
})
