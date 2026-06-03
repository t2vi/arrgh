// TDD contract tests for ADR 0031 new plugins.

import { describe, it, expect } from 'vitest'

import * as asurascans from '../../plugins/asurascans/src/index'
import * as wuxiaworld from '../../plugins/wuxiaworld/src/index'
import * as manga18fx  from '../../plugins/manga18fx/src/index'

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
  it('exports chapterText fn', () => expect(typeof wuxiaworld.chapterText).toBe('function'))
  it('no pages fn',            () => expect(wuxiaworld.pages).toBeUndefined())
})

// ── Manga18fx (manhwa, explicit) ──────────────────────────────────────────────

describe('manga18fx', () => {
  it('info.id is manga18fx',    () => expect(manga18fx.info.id).toBe('manga18fx'))
  it('name is non-empty',       () => expect(manga18fx.info.name.length).toBeGreaterThan(0))
  it('default_explicit true',   () => expect(manga18fx.info.default_explicit).toBe(true))

  it('content_types covers manhwa', () => expect(manga18fx.info.content_types).toContain('manhwa'))
  it('no manga in content_types',   () => expect(manga18fx.info.content_types).not.toContain('manga'))
  it('no manhua in content_types',  () => expect(manga18fx.info.content_types).not.toContain('manhua'))

  it('exports search fn',   () => expect(typeof manga18fx.search).toBe('function'))
  it('exports chapters fn', () => expect(typeof manga18fx.chapters).toBe('function'))
  it('exports pages fn',    () => expect(typeof manga18fx.pages).toBe('function'))
  it('no chapterText fn',   () => expect((manga18fx as any).chapterText).toBeUndefined())
})
