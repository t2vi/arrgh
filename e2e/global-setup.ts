import { chromium, FullConfig } from '@playwright/test'
import fs from 'fs'
import path from 'path'

const BASE = 'http://localhost:8080'
const ADMIN = { username: 'test-admin', password: 'testpassword123' }

export default async function globalSetup(_config: FullConfig) {
  await waitForServer()

  // Register the admin account (first registered user → auto-admin)
  const reg = await fetch(`${BASE}/api/auth/register`, {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify(ADMIN),
  })
  if (!reg.ok && reg.status !== 409 && reg.status !== 403) {
    throw new Error(`globalSetup: register failed ${reg.status} ${await reg.text()}`)
  }
  // 403 = users already exist (volume not cleaned between runs) — fall through to login

  // Log in to get the token
  const login = await fetch(`${BASE}/api/auth/login`, {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify(ADMIN),
  })
  if (!login.ok) throw new Error(`globalSetup: login failed ${login.status}`)
  const { token, role, allow_explicit } = await login.json() as { token: string; role: string; allow_explicit: boolean }

  // Save storageState via a browser page so localStorage is populated
  const authDir = path.resolve(__dirname, '.auth')
  fs.mkdirSync(authDir, { recursive: true })

  const browser = await chromium.launch()
  const context = await browser.newContext()
  const page = await context.newPage()
  await page.goto(`${BASE}/`)
  await page.evaluate(({ token, username, role, allow_explicit }) => {
    localStorage.setItem('arrgh_token', token)
    localStorage.setItem('arrgh_username', username)
    localStorage.setItem('arrgh_role', role)
    localStorage.setItem('arrgh_allow_explicit', String(allow_explicit))
  }, { token, username: ADMIN.username, role, allow_explicit })
  await context.storageState({ path: path.join(authDir, 'admin.json') })
  await browser.close()
}

async function waitForServer(ms = 60_000) {
  const deadline = Date.now() + ms
  while (Date.now() < deadline) {
    try {
      // Any HTTP response (even 401) means nginx+server are up
      await fetch(`${BASE}/api/auth/login`, { method: 'POST', headers: { 'Content-Type': 'application/json' }, body: '{}' })
      return
    } catch { /* not ready yet */ }
    await new Promise(r => setTimeout(r, 1000))
  }
  throw new Error('globalSetup: server did not become ready within 60s')
}
