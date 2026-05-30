// TDD: plugin-host Express routing tests.
// Requires: export `createApp(plugins: Map<string, PluginBundle>)` from index.ts
// All tests fail until createApp is extracted from the module-level boot logic.

import { describe, it, expect, vi, beforeEach } from 'vitest'
import request from 'supertest'
import type { PluginBundle, PluginInfo } from './index'
import { createApp } from './index'

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
