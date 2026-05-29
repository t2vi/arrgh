// TDD contract tests for ADR 0031 new plugins.
// All tests fail until the 5 new plugin directories are created.
// Implementation checklist per plugin:
//   - plugins/<name>/src/index.ts  — exports info, search, chapters, pages (or chapterText for novels)
//   - plugins/<name>/src/<name>.ts — implementation
//   - plugins/<name>/package.json  — with build script
//   - plugins/<name>/tsconfig.json

import { describe, it, expect } from 'vitest'

// ── Imports (will fail to resolve until plugin dirs are created) ──────────────

import * as mangafire  from '../../plugins/mangafire/src/index'
import * as asurascans from '../../plugins/asurascans/src/index'
import * as manhuafast from '../../plugins/manhuafast/src/index'
import * as wuxiaworld from '../../plugins/wuxiaworld/src/index'
import * as boxnovel   from '../../plugins/boxnovel/src/index'

// ── MangaFire (manga + manhwa + manhua) ───────────────────────────────────────

describe('mangafire', () => {
  it('info.id is mangafire',   () => expect(mangafire.info.id).toBe('mangafire'))
  it('name is non-empty',      () => expect(mangafire.info.name.length).toBeGreaterThan(0))
  it('default_explicit false', () => expect(mangafire.info.default_explicit).toBe(false))

  it('content_types covers manga',   () => expect(mangafire.info.content_types).toContain('manga'))
  it('content_types covers manhwa',  () => expect(mangafire.info.content_types).toContain('manhwa'))
  it('content_types covers manhua',  () => expect(mangafire.info.content_types).toContain('manhua'))

  it('exports search fn',   () => expect(typeof mangafire.search).toBe('function'))
  it('exports chapters fn', () => expect(typeof mangafire.chapters).toBe('function'))
  it('exports pages fn',    () => expect(typeof mangafire.pages).toBe('function'))

  // search must return items with required fields
  it('search result shape', async () => {
    // Mock fetch before calling search
    const mockFetch = vi.fn().mockResolvedValue({
      ok: true,
      text: async () => '<html></html>',
      json: async () => ({ results: [] }),
    })
    vi.stubGlobal('fetch', mockFetch)
    const results = await mangafire.search('test').catch(() => [])
    // Must return array (even if empty from mock)
    expect(Array.isArray(results)).toBe(true)
    vi.unstubAllGlobals()
  })
})

// ── AsuraScans (manhwa) ───────────────────────────────────────────────────────

describe('asurascans', () => {
  it('info.id is asurascans',  () => expect(asurascans.info.id).toBe('asurascans'))
  it('name is non-empty',      () => expect(asurascans.info.name.length).toBeGreaterThan(0))
  it('default_explicit false', () => expect(asurascans.info.default_explicit).toBe(false))

  it('content_types covers manhwa', () => expect(asurascans.info.content_types).toContain('manhwa'))
  it('no manga in content_types',   () => expect(asurascans.info.content_types).not.toContain('manga'))
  it('no manhua in content_types',  () => expect(asurascans.info.content_types).not.toContain('manhua'))

  it('exports search fn',   () => expect(typeof asurascans.search).toBe('function'))
  it('exports chapters fn', () => expect(typeof asurascans.chapters).toBe('function'))
  it('exports pages fn',    () => expect(typeof asurascans.pages).toBe('function'))
})

// ── ManhuaFast (manhua) ───────────────────────────────────────────────────────

describe('manhuafast', () => {
  it('info.id is manhuafast',  () => expect(manhuafast.info.id).toBe('manhuafast'))
  it('name is non-empty',      () => expect(manhuafast.info.name.length).toBeGreaterThan(0))
  it('default_explicit false', () => expect(manhuafast.info.default_explicit).toBe(false))

  it('content_types covers manhua', () => expect(manhuafast.info.content_types).toContain('manhua'))
  it('no manga in content_types',   () => expect(manhuafast.info.content_types).not.toContain('manga'))
  it('no manhwa in content_types',  () => expect(manhuafast.info.content_types).not.toContain('manhwa'))

  it('exports search fn',   () => expect(typeof manhuafast.search).toBe('function'))
  it('exports chapters fn', () => expect(typeof manhuafast.chapters).toBe('function'))
  it('exports pages fn',    () => expect(typeof manhuafast.pages).toBe('function'))
})

// ── WuxiaWorld (novel) ────────────────────────────────────────────────────────

describe('wuxiaworld', () => {
  it('info.id is wuxiaworld',  () => expect(wuxiaworld.info.id).toBe('wuxiaworld'))
  it('name is non-empty',      () => expect(wuxiaworld.info.name.length).toBeGreaterThan(0))
  it('default_explicit false', () => expect(wuxiaworld.info.default_explicit).toBe(false))

  it('content_types covers novel', () => expect(wuxiaworld.info.content_types).toContain('novel'))
  it('no manga in content_types',  () => expect(wuxiaworld.info.content_types).not.toContain('manga'))
  it('no manhwa in content_types', () => expect(wuxiaworld.info.content_types).not.toContain('manhwa'))

  it('exports search fn',      () => expect(typeof wuxiaworld.search).toBe('function'))
  it('exports chapters fn',    () => expect(typeof wuxiaworld.chapters).toBe('function'))
  // Novel plugins serve text, not page images
  it('exports chapterText fn', () => expect(typeof wuxiaworld.chapterText).toBe('function'))
  it('no pages fn',            () => expect(wuxiaworld.pages).toBeUndefined())
})

// ── BoxNovel (novel) ──────────────────────────────────────────────────────────

describe('boxnovel', () => {
  it('info.id is boxnovel',    () => expect(boxnovel.info.id).toBe('boxnovel'))
  it('name is non-empty',      () => expect(boxnovel.info.name.length).toBeGreaterThan(0))
  it('default_explicit false', () => expect(boxnovel.info.default_explicit).toBe(false))

  it('content_types covers novel', () => expect(boxnovel.info.content_types).toContain('novel'))
  it('no manga in content_types',  () => expect(boxnovel.info.content_types).not.toContain('manga'))
  it('no manhwa in content_types', () => expect(boxnovel.info.content_types).not.toContain('manhwa'))

  it('exports search fn',      () => expect(typeof boxnovel.search).toBe('function'))
  it('exports chapters fn',    () => expect(typeof boxnovel.chapters).toBe('function'))
  it('exports chapterText fn', () => expect(typeof boxnovel.chapterText).toBe('function'))
  it('no pages fn',            () => expect(boxnovel.pages).toBeUndefined())
})
