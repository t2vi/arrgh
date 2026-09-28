import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'

// spec: 009/FR-010, 009/FR-011 — getBrowser's single-connection reuse + reconnect
// contract (User Story 4). `vi.hoisted` is required here: `vi.mock`'s factory runs
// before this file's own top-level `const`s are initialized.
const { connectOverCDP } = vi.hoisted(() => ({ connectOverCDP: vi.fn() }))
vi.mock('playwright-core', () => ({
  chromium: { connectOverCDP },
}))

function fakeBrowser() {
  const handlers: Record<string, () => void> = {}
  return {
    isConnected: vi.fn().mockReturnValue(true),
    on: vi.fn((event: string, cb: () => void) => {
      handlers[event] = cb
    }),
    trigger(event: string) {
      handlers[event]?.()
    },
  }
}

describe('getBrowser', () => {
  const originalFetch = global.fetch

  beforeEach(() => {
    vi.resetModules()
    connectOverCDP.mockReset()
  })

  afterEach(() => {
    global.fetch = originalFetch
    vi.unstubAllEnvs()
  })

  it('throws immediately when CLOAKBROWSER_WS_URL is not configured', async () => {
    vi.stubEnv('CLOAKBROWSER_WS_URL', '')
    const { getBrowser } = await import('./index')
    await expect(getBrowser()).rejects.toThrow(/CLOAKBROWSER_WS_URL is not set/)
  })

  it('connects once via CDP and reuses the cached instance on a later call', async () => {
    vi.stubEnv('CLOAKBROWSER_WS_URL', 'http://cloak.test')
    global.fetch = vi.fn().mockResolvedValue({
      json: async () => ({ webSocketDebuggerUrl: 'ws://0.0.0.0:1234/devtools/browser/abc' }),
    }) as unknown as typeof fetch
    const browser = fakeBrowser()
    connectOverCDP.mockResolvedValue(browser)

    const { getBrowser } = await import('./index')
    const first = await getBrowser()
    const second = await getBrowser()

    expect(first).toBe(browser)
    expect(second).toBe(browser)
    expect(connectOverCDP).toHaveBeenCalledTimes(1)
    expect(global.fetch).toHaveBeenCalledTimes(1)
  })

  it('reconnects on the next call after the cached connection disconnects', async () => {
    vi.stubEnv('CLOAKBROWSER_WS_URL', 'http://cloak.test')
    global.fetch = vi.fn().mockResolvedValue({
      json: async () => ({ webSocketDebuggerUrl: 'ws://0.0.0.0:1234/devtools/browser/abc' }),
    }) as unknown as typeof fetch
    const first = fakeBrowser()
    const second = fakeBrowser()
    connectOverCDP.mockResolvedValueOnce(first).mockResolvedValueOnce(second)

    const { getBrowser } = await import('./index')
    const a = await getBrowser()
    a.trigger('disconnected')
    const b = await getBrowser()

    expect(a).toBe(first)
    expect(b).toBe(second)
    expect(connectOverCDP).toHaveBeenCalledTimes(2)
  })
})
