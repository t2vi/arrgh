import { chromium } from 'playwright-core'
import { writeFileSync, mkdirSync } from 'node:fs'
import { join, dirname } from 'node:path'
import { fileURLToPath } from 'node:url'

const __dirname = dirname(fileURLToPath(import.meta.url))
const SNAPSHOTS_DIR = join(__dirname, '../snapshots')

export function saveRaw(source: string, op: string, label: string, content: string, ext: 'html' | 'json') {
  const dir = join(SNAPSHOTS_DIR, source)
  mkdirSync(dir, { recursive: true })
  writeFileSync(join(dir, `${op}-${slugify(label)}.${ext}`), content, 'utf8')
}

export function slugify(s: string) {
  return s.toLowerCase().replace(/[^a-z0-9]+/g, '-').replace(/^-|-$/g, '').slice(0, 60)
}

// Wrap global fetch to capture the first response per call as a raw snapshot file.
// Use for non-CF sources (mangadex, mangapill, royalroad, wuxiaworld).
export async function captureFetch<T>(
  source: string, op: string, label: string,
  fn: () => Promise<T>
): Promise<T> {
  let captured = false
  const origFetch = globalThis.fetch as typeof fetch
  ;(globalThis as Record<string, unknown>).fetch = async (...args: Parameters<typeof fetch>) => {
    const res = await origFetch(...args)
    if (!captured) {
      captured = true
      const clone = res.clone()
      const text = await clone.text()
      const ext = text.trimStart().startsWith('{') || text.trimStart().startsWith('[') ? 'json' : 'html'
      saveRaw(source, op, label, text, ext)
    }
    return res
  }
  try {
    return await fn()
  } finally {
    ;(globalThis as Record<string, unknown>).fetch = origFetch
  }
}

// ── CloakBrowser context ──────────────────────────────────────────────────────

// Matches the PluginContext shape all CF plugins expect (duck-typed)
export interface PluginContext {
  getBrowser(): Promise<{
    isConnected(): boolean
    newContext(opts?: Record<string, unknown>): Promise<{
      close(): Promise<void>
      addCookies(cookies: unknown[]): Promise<void>
      newPage(): Promise<{
        goto(url: string, opts?: unknown): Promise<unknown>
        content(): Promise<string>
        evaluate<T>(fn: unknown, ...args: unknown[]): Promise<T>
        close(): Promise<void>
      }>
    }>
  }>
  logger: typeof console
}

export interface BrowserConn {
  makeContext(source: string, op: string, label: string): PluginContext
  close(): Promise<void>
}

export async function connectBrowser(): Promise<BrowserConn | null> {
  const endpointUrl = process.env.CLOAK_WS_URL
  if (!endpointUrl) return null
  try {
    // CloakBrowser runs in Docker and advertises its internal hostname in the WS URL.
    // Fetch the WS URL from the CDP discovery endpoint, then rewrite the hostname to localhost.
    const versionRes = await fetch(`${endpointUrl}/json/version`)
    const { webSocketDebuggerUrl } = await versionRes.json() as { webSocketDebuggerUrl: string }
    const wsUrl = webSocketDebuggerUrl.replace(/^ws:\/\/[^/]+/, `ws://localhost:${new URL(endpointUrl).port || 80}`)
    const browser = await chromium.connectOverCDP(wsUrl)
    return {
      makeContext(source: string, op: string, label: string): PluginContext {
        let captured = false
        return {
          getBrowser: async () => ({
            isConnected: () => browser.isConnected(),
            newContext: async (opts?: Record<string, unknown>) => {
              const bctx = await browser.newContext(opts)
              return {
                close: () => bctx.close(),
                addCookies: (cookies: unknown[]) => bctx.addCookies(cookies as Parameters<typeof bctx.addCookies>[0]),
                newPage: async () => {
                  const page = await bctx.newPage()
                  let lastUrl = ''
                  return {
                    goto: async (url: string, o?: unknown) => {
                      lastUrl = url
                      return page.goto(url, o as Parameters<typeof page.goto>[1])
                    },
                    content: async () => {
                      const html = await page.content()
                      if (!captured) {
                        captured = true
                        saveRaw(source, op, label, html, 'html')
                      }
                      return html
                    },
                    evaluate: page.evaluate.bind(page),
                    close: () => page.close(),
                  }
                },
              }
            },
          }),
          logger: console,
        }
      },
      close: () => browser.close(),
    }
  } catch {
    return null
  }
}
