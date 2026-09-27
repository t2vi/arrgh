// TDD: plugin-host Express routing tests.
// Requires: export `createApp(plugins: Map<string, PluginBundle>)` from index.ts
// All tests fail until createApp is extracted from the module-level boot logic.

import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest'
import { createHash } from 'node:crypto'
import request from 'supertest'
import type { PluginBundle, PluginInfo } from './index'
import {
  createApp, rewriteCdpHost, PluginRegistry, compareVersions, pickActive, loadDir,
  ORIGIN_BUNDLED, ORIGIN_DOWNLOADED, PLUGIN_PROTOCOL,
} from './index'

// ── Mock plugin factories ─────────────────────────────────────────────────────

function makePlugin(overrides: Partial<PluginBundle> & { info: PluginInfo }): PluginBundle {
  return {
    search:    vi.fn().mockResolvedValue([{ id: 'r1', title: 'Mock Result' }]),
    chapters:  vi.fn().mockResolvedValue([{ id: 'ch1', number: 1, chapter_format: 'pages' }]),
    pages:     vi.fn().mockResolvedValue(['https://cdn.example.com/p1.jpg']),
    ...overrides,
  }
}

const MANGA_PLUGIN = makePlugin({
  info: { id: 'mock-manga', name: 'Mock Manga', default_explicit: false, content_types: ['manga'] },
})

const NOVEL_PLUGIN = makePlugin({
  info: { id: 'mock-novel', name: 'Mock Novel', default_explicit: false, content_types: ['novel'] },
  pages: undefined, // novel plugins serve text, not pages
  chapterText: vi.fn().mockResolvedValue('# Chapter 1\n\nOnce upon a time…'),
})

const EXPLICIT_PLUGIN = makePlugin({
  info: { id: 'mock-explicit', name: 'Mock Explicit', default_explicit: true, content_types: ['manga'] },
})

// ── Tests ─────────────────────────────────────────────────────────────────────

describe('GET /plugins', () => {
  it('returns all loaded plugins', async () => {
    const registry = new Map([
      ['mock-manga', MANGA_PLUGIN],
      ['mock-novel', NOVEL_PLUGIN],
    ])
    const app = createApp(registry)
    const res = await request(app).get('/plugins')
    expect(res.status).toBe(200)
    expect(res.body).toHaveLength(2)
    expect(res.body[0]).toMatchObject({ id: expect.any(String), name: expect.any(String), default_explicit: expect.any(Boolean), content_types: expect.any(Array) })
  })

  it('returns empty array when no plugins loaded', async () => {
    const app = createApp(new Map())
    const res = await request(app).get('/plugins')
    expect(res.status).toBe(200)
    expect(res.body).toEqual([])
  })
})

describe('GET /:plugin/info', () => {
  it('returns plugin info for known plugin', async () => {
    const app = createApp(new Map([['mock-manga', MANGA_PLUGIN]]))
    const res = await request(app).get('/mock-manga/info')
    expect(res.status).toBe(200)
    expect(res.body.id).toBe('mock-manga')
    expect(res.body.default_explicit).toBe(false)
    expect(Array.isArray(res.body.content_types)).toBe(true)
  })

  it('returns 404 for unknown plugin', async () => {
    const app = createApp(new Map())
    const res = await request(app).get('/nonexistent/info')
    expect(res.status).toBe(404)
  })
})

describe('GET /:plugin/search', () => {
  it('calls plugin.search and returns results', async () => {
    const app = createApp(new Map([['mock-manga', MANGA_PLUGIN]]))
    const res = await request(app).get('/mock-manga/search?q=naruto')
    expect(res.status).toBe(200)
    expect(res.body).toEqual([{ id: 'r1', title: 'Mock Result' }])
    expect(MANGA_PLUGIN.search).toHaveBeenCalledWith('naruto')
  })

  it('returns empty array when q is missing', async () => {
    const app = createApp(new Map([['mock-manga', MANGA_PLUGIN]]))
    const res = await request(app).get('/mock-manga/search')
    expect(res.status).toBe(200)
    expect(res.body).toEqual([])
  })

  it('returns 502 on plugin.search error', async () => {
    const errorPlugin = makePlugin({
      info: { id: 'err', name: 'Error', default_explicit: false, content_types: ['manga'] },
      search: vi.fn().mockRejectedValue(new Error('scrape failed')),
    })
    const app = createApp(new Map([['err', errorPlugin]]))
    const res = await request(app).get('/err/search?q=test')
    expect(res.status).toBe(502)
  })

  it('returns 404 for unknown plugin', async () => {
    const app = createApp(new Map())
    const res = await request(app).get('/missing/search?q=test')
    expect(res.status).toBe(404)
  })
})

describe('GET /:plugin/manga/:id/chapters', () => {
  it('calls plugin.chapters and returns results', async () => {
    const app = createApp(new Map([['mock-manga', MANGA_PLUGIN]]))
    const res = await request(app).get('/mock-manga/manga/series-id-123/chapters')
    expect(res.status).toBe(200)
    expect(res.body).toEqual([{ id: 'ch1', number: 1, chapter_format: 'pages' }])
    expect(MANGA_PLUGIN.chapters).toHaveBeenCalledWith('series-id-123', expect.any(Array))
  })

  it('url-decodes the source id', async () => {
    const app = createApp(new Map([['mock-manga', MANGA_PLUGIN]]))
    const encodedId = encodeURIComponent('id with spaces')
    const res = await request(app).get(`/mock-manga/manga/${encodedId}/chapters`)
    expect(res.status).toBe(200)
    expect(MANGA_PLUGIN.chapters).toHaveBeenCalledWith('id with spaces', expect.any(Array))
  })

  it('returns 502 on plugin error', async () => {
    const errorPlugin = makePlugin({
      info: { id: 'err', name: 'Error', default_explicit: false, content_types: ['manga'] },
      chapters: vi.fn().mockRejectedValue(new Error('chapters failed')),
    })
    const app = createApp(new Map([['err', errorPlugin]]))
    const res = await request(app).get('/err/manga/id/chapters')
    expect(res.status).toBe(502)
  })
})

describe('GET /:plugin/chapter/:id/pages', () => {
  it('calls plugin.pages and returns URLs', async () => {
    const app = createApp(new Map([['mock-manga', MANGA_PLUGIN]]))
    const res = await request(app).get('/mock-manga/chapter/ch-001/pages')
    expect(res.status).toBe(200)
    expect(res.body).toEqual(['https://cdn.example.com/p1.jpg'])
    expect(MANGA_PLUGIN.pages).toHaveBeenCalledWith('ch-001')
  })

  it('returns 404 when plugin has no pages fn (novel plugin)', async () => {
    const app = createApp(new Map([['mock-novel', NOVEL_PLUGIN]]))
    const res = await request(app).get('/mock-novel/chapter/ch-001/pages')
    expect(res.status).toBe(404)
  })

  it('returns 404 for unknown plugin', async () => {
    const app = createApp(new Map())
    const res = await request(app).get('/missing/chapter/ch-001/pages')
    expect(res.status).toBe(404)
  })
})

describe('GET /:plugin/chapter/:id/text', () => {
  it('calls plugin.chapterText and returns markdown', async () => {
    const app = createApp(new Map([['mock-novel', NOVEL_PLUGIN]]))
    const res = await request(app).get('/mock-novel/chapter/ch-001/text')
    expect(res.status).toBe(200)
    expect(res.text).toContain('Chapter 1')
    expect(NOVEL_PLUGIN.chapterText).toHaveBeenCalledWith('ch-001')
  })

  it('returns 404 when plugin has no chapterText fn (manga plugin)', async () => {
    const app = createApp(new Map([['mock-manga', MANGA_PLUGIN]]))
    const res = await request(app).get('/mock-manga/chapter/ch-001/text')
    expect(res.status).toBe(404)
  })
})

describe('POST /plugins/install', () => {
  it('returns 400 when url missing', async () => {
    const app = createApp(new Map())
    const res = await request(app).post('/plugins/install').send({})
    expect(res.status).toBe(400)
  })
})

describe('DELETE /plugins/:id', () => {
  it('returns 403 for bundled (non-community) plugin', async () => {
    const app = createApp(new Map([['mock-manga', MANGA_PLUGIN]]), new Set()) // empty communityIds
    const res = await request(app).delete('/plugins/mock-manga')
    expect(res.status).toBe(403)
  })

  it('returns 204 and removes community plugin', async () => {
    const registry = new Map([['mock-manga', MANGA_PLUGIN]])
    const communityIds = new Set(['mock-manga'])
    const app = createApp(registry, communityIds)
    const res = await request(app).delete('/plugins/mock-manga')
    expect(res.status).toBe(204)
    expect(registry.has('mock-manga')).toBe(false)
  })
})

// ── rewriteCdpHost ────────────────────────────────────────────────────────────

describe('rewriteCdpHost', () => {
  it('rewrites 0.0.0.0 to localhost for local dev', () => {
    const result = rewriteCdpHost(
      'ws://0.0.0.0:3001/devtools/browser/abc123',
      'http://localhost:3001',
    )
    expect(result).toBe('ws://localhost:3001/devtools/browser/abc123')
  })

  it('rewrites internal hostname to Docker service name', () => {
    const result = rewriteCdpHost(
      'ws://0.0.0.0:3000/devtools/browser/abc123',
      'http://cloakbrowser:3000',
    )
    expect(result).toBe('ws://cloakbrowser:3000/devtools/browser/abc123')
  })

  it('preserves path after host rewrite', () => {
    const result = rewriteCdpHost(
      'ws://172.17.0.2:3000/devtools/browser/uuid-here/path',
      'http://cloakbrowser:3000',
    )
    expect(result).toBe('ws://cloakbrowser:3000/devtools/browser/uuid-here/path')
  })
})

// ── Bundle watcher (spec 020): removed bundle files are unloaded ──────────────

import { onBundleChange } from './index'
import * as fsm from 'node:fs'
import * as os from 'node:os'
import * as pathm from 'node:path'

describe('onBundleChange', () => {
  it('loads a new bundle file, and unloads it when the file is removed', async () => {
    const dir = fsm.mkdtempSync(pathm.join(os.tmpdir(), 'bundles-'))
    const file = pathm.join(dir, 'tmpplugin.js')
    fsm.writeFileSync(file, `module.exports = { info: { id: 'tmpplugin', name: 'Tmp', content_types: ['manga'] } }`)
    const reg = new PluginRegistry()

    await onBundleChange(reg, file, ORIGIN_DOWNLOADED)
    expect(reg.active.has('tmpplugin')).toBe(true)
    expect(reg.origin('tmpplugin')).toBe(ORIGIN_DOWNLOADED)

    fsm.unlinkSync(file)
    await onBundleChange(reg, file, ORIGIN_DOWNLOADED)
    expect(reg.active.has('tmpplugin')).toBe(false)
  })
})

// ── Plugin updates (spec 031 phase A) ─────────────────────────────────────────

const bundleSrc = (id: string, version?: string, extra = '') =>
  `module.exports = { info: { id: '${id}', name: '${id}', ${version ? `version: '${version}', ` : ''}default_explicit: false, content_types: ['manga'] }, search: async () => ['${id}@${version ?? 'unknown'}'], chapters: async () => [] ${extra} }`
const sha = (s: string) => createHash('sha256').update(s).digest('hex')
const loaded = (id: string, version?: string, file?: string) =>
  ({ bundle: makePlugin({ info: { id, name: id, version, default_explicit: false, content_types: ['manga'] } }), file })

// spec: 031/FR-008
describe('compareVersions / pickActive', () => {
  it('compares numerically, not as strings', () => {
    expect(compareVersions('1.10.0', '1.9.0')).toBeGreaterThan(0)
    expect(compareVersions('1.0.0', '1.0.0')).toBe(0)
    expect(compareVersions('1.0', '1.0.1')).toBeLessThan(0)
    expect(compareVersions('2.0.0-beta.1', '1.9.9')).toBeGreaterThan(0)
  })

  it('downloaded beats bundled', () => {
    const d = loaded('p', '1.1.0')
    expect(pickActive({ bundled: loaded('p', '1.0.0'), downloaded: d })).toBe(d)
  })

  it('bundled wins when it is newer than the download (app upgraded past it)', () => {
    const b = loaded('p', '1.2.0')
    expect(pickActive({ bundled: b, downloaded: loaded('p', '1.1.0') })).toBe(b)
  })

  it('a download with unknown version still wins (explicit admin action)', () => {
    const d = loaded('p')
    expect(pickActive({ bundled: loaded('p', '1.0.0'), downloaded: d })).toBe(d)
  })
})

describe('GET /host and GET /plugins versions', () => {
  // spec: 031/FR-006
  it('reports the supported plugin protocol', async () => {
    const res = await request(createApp(new PluginRegistry())).get('/host')
    expect(res.body).toEqual({ protocol: PLUGIN_PROTOCOL })
  })

  // spec: 031/FR-003, 031/FR-010
  it('lists the version the loaded bundle reports, its origin and whether a bundled copy exists', async () => {
    const reg = new PluginRegistry()
    reg.put('a', ORIGIN_BUNDLED, loaded('a', '1.0.0'))
    reg.put('a', ORIGIN_DOWNLOADED, loaded('a', '1.1.0'))
    reg.put('b', ORIGIN_BUNDLED, loaded('b'))
    const res = await request(createApp(reg)).get('/plugins')
    const by = Object.fromEntries(res.body.map((p: { id: string }) => [p.id, p]))
    expect(by.a).toMatchObject({ version: '1.1.0', origin: ORIGIN_DOWNLOADED, has_bundled: true, is_community: true })
    expect(by.b).toMatchObject({ version: null, origin: ORIGIN_BUNDLED, has_bundled: true, is_community: false })
  })
})

describe('POST /plugins/install (verified, id-keyed)', () => {
  let dir: string
  let reg: PluginRegistry
  let app: ReturnType<typeof createApp>
  const serve = (body: string, ok = true) =>
    vi.stubGlobal('fetch', vi.fn(async () => new Response(body, { status: ok ? 200 : 500 })))
  const install = (body: Record<string, unknown>) => request(app).post('/plugins/install').send(body)
  const URL_ = 'https://example.com/releases/p.js'

  beforeEach(() => {
    dir = fsm.mkdtempSync(pathm.join(os.tmpdir(), 'community-'))
    reg = new PluginRegistry()
    reg.put('p', ORIGIN_BUNDLED, loaded('p', '1.0.0'))
    app = createApp(reg, undefined, { communityDir: dir })
  })
  afterEach(() => vi.unstubAllGlobals())

  const stillBundled = () => {
    expect(reg.origin('p')).toBe(ORIGIN_BUNDLED)
    expect(reg.active.get('p')?.info.version).toBe('1.0.0')
    expect(fsm.readdirSync(dir)).toEqual([])
  }

  // spec: 031/FR-005, 031/FR-007, 031/FR-010
  it('verifies, saves as <id>.js and serves the new version without a restart', async () => {
    const code = bundleSrc('p', '1.1.0')
    serve(code)
    const res = await install({ id: 'p', url: URL_, sha256: sha(code), protocol: 1 })
    expect(res.status).toBe(201)
    expect(res.body).toEqual({ id: 'p', version: '1.1.0', origin: ORIGIN_DOWNLOADED })
    expect(fsm.readdirSync(dir)).toEqual(['p.js'])
    expect((await request(app).get('/p/search?q=x')).body).toEqual(['p@1.1.0'])
  })

  // spec: 031/FR-007
  it('the download persists: a fresh host over the same folders loads it', async () => {
    const code = bundleSrc('p', '1.1.0')
    serve(code)
    await install({ id: 'p', url: URL_, sha256: sha(code) })
    const fresh = new PluginRegistry()
    fresh.put('p', ORIGIN_BUNDLED, loaded('p', '1.0.0'))
    await loadDir(fresh, dir, ORIGIN_DOWNLOADED)
    expect(fresh.origin('p')).toBe(ORIGIN_DOWNLOADED)
    expect(fresh.active.get('p')?.info.version).toBe('1.1.0')
  })

  // spec: 031/FR-005
  it('400 without a checksum, nothing downloaded', async () => {
    serve(bundleSrc('p', '1.1.0'))
    const res = await install({ id: 'p', url: URL_ })
    expect(res.status).toBe(400)
    expect(fetch).not.toHaveBeenCalled()
    stillBundled()
  })

  it('400 without id or url', async () => {
    expect((await install({ url: URL_, sha256: 'x' })).status).toBe(400)
    expect((await install({ id: 'p', sha256: 'x' })).status).toBe(400)
  })

  // spec: 031/FR-005, 031/FR-012
  it('422 on checksum mismatch; previous version keeps serving', async () => {
    serve(bundleSrc('p', '1.1.0'))
    const res = await install({ id: 'p', url: URL_, sha256: sha('something else') })
    expect(res.status).toBe(422)
    expect(res.body.error).toMatch(/checksum mismatch/)
    stillBundled()
  })

  // spec: 031/FR-006
  it('422 when the bundle needs a newer protocol, before downloading', async () => {
    serve(bundleSrc('p', '2.0.0'))
    const res = await install({ id: 'p', url: URL_, sha256: 'x', protocol: PLUGIN_PROTOCOL + 1 })
    expect(res.status).toBe(422)
    expect(res.body.error).toMatch(/protocol/)
    expect(fetch).not.toHaveBeenCalled()
    stillBundled()
  })

  // spec: 031/FR-012
  it('422 when the bundle throws on load; previous version keeps serving', async () => {
    const code = `throw new Error('boom')`
    serve(code)
    const res = await install({ id: 'p', url: URL_, sha256: sha(code) })
    expect(res.status).toBe(422)
    expect(res.body.error).toMatch(/boom/)
    stillBundled()
  })

  // spec: 031/FR-012
  it('422 when the bundle reports a different id', async () => {
    const code = bundleSrc('other', '1.1.0')
    serve(code)
    const res = await install({ id: 'p', url: URL_, sha256: sha(code) })
    expect(res.status).toBe(422)
    expect(res.body.error).toMatch(/other/)
    stillBundled()
    expect(reg.active.has('other')).toBe(false)
  })

  // spec: 031/FR-012
  it('502 when the download fails', async () => {
    serve('', false)
    const res = await install({ id: 'p', url: URL_, sha256: 'x' })
    expect(res.status).toBe(502)
    stillBundled()
  })
})

describe('DELETE /plugins/:id (revert / remove)', () => {
  let dir: string
  beforeEach(() => { dir = fsm.mkdtempSync(pathm.join(os.tmpdir(), 'community-')) })
  const file = (name: string) => { const f = pathm.join(dir, name); fsm.writeFileSync(f, ''); return f }

  // spec: 031/FR-009
  it('reverts a download to the bundled version and deletes the file', async () => {
    const reg = new PluginRegistry()
    reg.put('p', ORIGIN_BUNDLED, loaded('p', '1.0.0'))
    reg.put('p', ORIGIN_DOWNLOADED, loaded('p', '1.1.0', file('p.js')))
    const res = await request(createApp(reg)).delete('/plugins/p')
    expect(res.status).toBe(200)
    expect(res.body).toEqual({ id: 'p', version: '1.0.0', origin: ORIGIN_BUNDLED })
    expect(reg.active.get('p')?.info.version).toBe('1.0.0')
    expect(fsm.existsSync(pathm.join(dir, 'p.js'))).toBe(false)
  })

  // spec: 031/FR-009
  it('unloads a downloaded plugin that has no bundled version (204)', async () => {
    const reg = new PluginRegistry()
    reg.put('p', ORIGIN_DOWNLOADED, loaded('p', '1.0.0', file('p.js')))
    const res = await request(createApp(reg)).delete('/plugins/p')
    expect(res.status).toBe(204)
    expect(reg.active.has('p')).toBe(false)
  })

  // spec: 031/FR-009
  it('removes only the file that plugin registered (novelfull never touches novelfullnet.js)', async () => {
    const reg = new PluginRegistry()
    reg.put('novelfull', ORIGIN_DOWNLOADED, loaded('novelfull', '1.0.0', file('novelfull-legacy.js')))
    reg.put('novelfullnet', ORIGIN_DOWNLOADED, loaded('novelfullnet', '1.0.0', file('novelfullnet.js')))
    await request(createApp(reg)).delete('/plugins/novelfull')
    expect(fsm.readdirSync(dir)).toEqual(['novelfullnet.js'])
    expect(reg.active.has('novelfullnet')).toBe(true)
  })

  it('404 for an unknown id', async () => {
    expect((await request(createApp(new PluginRegistry())).delete('/plugins/nope')).status).toBe(404)
  })
})

describe('plugin call timeout (GH #159)', () => {
  const never = () => new Promise<never>(() => {})
  const HUNG = makePlugin({
    info: { id: 'hung', name: 'Hung', default_explicit: false, content_types: ['manga'] },
    search: vi.fn(never), chapters: vi.fn(never), pages: vi.fn(never),
    trending: vi.fn(never), meta: vi.fn(never), chapterText: vi.fn(never), cover: vi.fn(never),
  })

  it.each([
    '/hung/search?q=x',
    '/hung/trending',
    '/hung/manga/m1/meta',
    '/hung/manga/m1/chapters',
    '/hung/chapter/c1/pages',
    '/hung/chapter/c1/text',
    '/hung/cover?url=https://x/y.jpg',
  ])('%s returns 504 instead of hanging', async (url) => {
    const app = createApp(new Map([['hung', HUNG]]), new Set(), { callTimeoutMs: 50 })
    const res = await request(app).get(url)
    expect(res.status).toBe(504)
    expect(res.body.error).toMatch(/timed out after 50ms/)
  }, 2000)
})
