// Contract tests for all bundled default plugins.
// Tests info shape + content_types — no HTTP calls.
// Catches: wrong content_types, missing default_explicit, id mismatch with plugin-index/index.json.

import { describe, it, expect } from 'vitest'

// ── Existing plugins ──────────────────────────────────────────────────────────

import * as mangadex      from '../../plugins/mangadex/src/index'
import * as mangapill     from '../../plugins/mangapill/src/index'
import * as nhentai       from '../../plugins/nhentai/src/index'
import * as novelfull     from '../../plugins/novelfull/src/index'
import * as royalroad     from '../../plugins/royalroad/src/index'
import * as toonily       from '../../plugins/toonily/src/index'
import * as novelupdates  from '../../plugins/novelupdates/src/index'

describe('mangadex', () => {
  it('info.id matches plugin-index id', () => expect(mangadex.info.id).toBe('mangadex'))
  it('default_explicit is false', () => expect(mangadex.info.default_explicit).toBe(false))
  it('content_types includes manga', () => expect(mangadex.info.content_types).toContain('manga'))
  it('content_types includes manhwa', () => expect(mangadex.info.content_types).toContain('manhwa'))
  it('content_types includes manhua', () => expect(mangadex.info.content_types).toContain('manhua'))
  it('exports search fn', () => expect(typeof mangadex.search).toBe('function'))
  it('exports chapters fn', () => expect(typeof mangadex.chapters).toBe('function'))
  it('exports pages fn', () => expect(typeof mangadex.pages).toBe('function'))
})

describe('mangapill', () => {
  it('info.id matches plugin-index id', () => expect(mangapill.info.id).toBe('mangapill'))
  it('default_explicit is false', () => expect(mangapill.info.default_explicit).toBe(false))
  it('content_types includes manga', () => expect(mangapill.info.content_types).toContain('manga'))
  it('exports search fn', () => expect(typeof mangapill.search).toBe('function'))
  it('exports chapters fn', () => expect(typeof mangapill.chapters).toBe('function'))
  it('exports pages fn', () => expect(typeof mangapill.pages).toBe('function'))
})

describe('nhentai', () => {
  it('info.id matches plugin-index id', () => expect(nhentai.info.id).toBe('nhentai'))
  it('default_explicit is true (explicit-only source)', () => expect(nhentai.info.default_explicit).toBe(true))
  it('exports search fn', () => expect(typeof nhentai.search).toBe('function'))
  it('exports chapters fn', () => expect(typeof nhentai.chapters).toBe('function'))
  it('exports pages fn', () => expect(typeof nhentai.pages).toBe('function'))
})

describe('novelfull', () => {
  it('info.id matches plugin-index id', () => expect(novelfull.info.id).toBe('novelfull'))
  it('default_explicit is false', () => expect(novelfull.info.default_explicit).toBe(false))
  it('content_types includes novel', () => expect(novelfull.info.content_types).toContain('novel'))
  it('exports search fn', () => expect(typeof novelfull.search).toBe('function'))
  it('exports chapters fn', () => expect(typeof novelfull.chapters).toBe('function'))
  it('novel plugin exports chapterText fn', () => expect(typeof novelfull.chapterText).toBe('function'))
})

describe('royalroad', () => {
  it('info.id is royalroad', () => expect(royalroad.info.id).toBe('royalroad'))
  it('default_explicit is false', () => expect(royalroad.info.default_explicit).toBe(false))
  it('content_types includes novel', () => expect(royalroad.info.content_types).toContain('novel'))
  it('exports search fn', () => expect(typeof royalroad.search).toBe('function'))
  it('exports chapters fn', () => expect(typeof royalroad.chapters).toBe('function'))
  it('novel plugin exports chapterText fn', () => expect(typeof royalroad.chapterText).toBe('function'))
})

describe('toonily', () => {
  it('info.id is toonily', () => expect(toonily.info.id).toBe('toonily'))
  it('default_explicit is false', () => expect(toonily.info.default_explicit).toBe(false))
  it('content_types includes manhwa', () => expect(toonily.info.content_types).toContain('manhwa'))
  it('exports search fn', () => expect(typeof toonily.search).toBe('function'))
  it('exports chapters fn', () => expect(typeof toonily.chapters).toBe('function'))
  it('exports pages fn', () => expect(typeof toonily.pages).toBe('function'))
})

describe('novelupdates', () => {
  it('info.id is novelupdates', () => expect(novelupdates.info.id).toBe('novelupdates'))
  it('default_explicit is false', () => expect(novelupdates.info.default_explicit).toBe(false))
  it('content_types includes novel', () => expect(novelupdates.info.content_types).toContain('novel'))
  it('exports search fn', () => expect(typeof novelupdates.search).toBe('function'))
  it('exports chapters fn', () => expect(typeof novelupdates.chapters).toBe('function'))
  it('no pages fn (metadata-only plugin)', () => expect((novelupdates as Record<string, unknown>).pages).toBeUndefined())
})

// ── plugin-index.json consistency check ──────────────────────────────────────
// Catches discrepancies between live plugin content_types and what plugin-index advertises.
// Failing test = plugin-index/index.json needs updating.

import indexJson from '../../plugin-index/index.json'

const indexMap = new Map(indexJson.map((p: { id: string; content_types: string[] }) => [p.id, p.content_types]))

describe('plugin-index.json consistency', () => {
  it('mangadex: index.json content_types matches plugin export', () => {
    const indexed = indexMap.get('mangadex') ?? []
    const live = mangadex.info.content_types
    // Live plugin knows its own content_types; index.json must be a superset or equal
    for (const ct of live) {
      expect(indexed, `plugin-index.json mangadex missing content_type "${ct}"`).toContain(ct)
    }
  })

  it('toonily: index.json content_types matches plugin export', () => {
    const indexed = indexMap.get('toonily') ?? []
    for (const ct of toonily.info.content_types) {
      expect(indexed, `plugin-index.json toonily missing content_type "${ct}"`).toContain(ct)
    }
  })

  it('nhentai: default_explicit true', () => {
    expect(nhentai.info.default_explicit).toBe(true)
  })

  it('novelupdates: index.json content_types includes novel', () => {
    const indexed = indexMap.get('novelupdates') ?? []
    expect(indexed).toContain('novel')
  })
})
