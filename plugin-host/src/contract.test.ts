// Contract tests for plugin-index/index.json itself — no HTTP calls, no plugin source imports.
// Per-plugin fixture/behavior/contract tests live in each plugin's own repo
// (t2vi/arrgh-plugin-<id>, spec 031 phase C). plugins/ no longer exists in this monorepo except
// plugins/fixture/ (e2e-only); the image fetches every bundled plugin from its own repo's
// published release at build time (spec 031 phase D, scripts/fetch-plugin-bundles.mjs).

import { describe, it, expect } from 'vitest'

// ── plugin-index.json consistency check ──────────────────────────────────────
// Catches discrepancies that don't depend on importing a specific plugin's live code.

import indexJson from '../../plugin-index/index.json'

const indexMap = new Map(indexJson.map((p: { id: string; content_types: string[] }) => [p.id, p.content_types]))

describe('plugin-index.json consistency', () => {
  it('novelupdates: index.json content_types includes novel', () => {
    const indexed = indexMap.get('novelupdates') ?? []
    expect(indexed).toContain('novel')
  })

  it('royalroad in plugin-index as bundled novel plugin (ADR 0034 supersedes 0024)', () => {
    const rr = indexJson.find((p: { id: string }) => p.id === 'royalroad')
    expect(rr?.bundled).toBe(true)
    expect(rr?.content_types).toEqual(['novel'])
  })

  it('manhuafast not in plugin-index (removed — CF managed challenge)', () => {
    expect(indexMap.has('manhuafast')).toBe(false)
  })

  it('boxnovel not in plugin-index (removed — domain parked)', () => {
    expect(indexMap.has('boxnovel')).toBe(false)
  })

  it('never marks the e2e fixture plugin bundled (it never ships in the image)', () => {
    expect(indexMap.has('fixture')).toBe(false)
  })
})

// spec: 031/FR-010, 031/FR-001, 031/FR-016 (phase D)
// The image now fetches every bundled entry's download_url and verifies both its sha256 and its
// bundle's own info.version at build time (scripts/fetch-plugin-bundles.mjs) — that IS the
// per-plugin version/checksum check, run for real against the network, not duplicated here.
describe('plugins report their own version (spec 031)', () => {
  const index = indexJson as { id: string; version: string; bundled?: boolean; sha256?: string | null; protocol?: number }[]

  it('every index entry carries sha256 and protocol keys', () => {
    for (const e of index) {
      expect(e, e.id).toHaveProperty('sha256')
      expect(e.protocol, e.id).toBeGreaterThanOrEqual(1)
    }
  })

  // spec: 031/FR-013
  it('every entry with a download_url has a 64-hex sha256 and a protocol', () => {
    const withDownload = index.filter((e) => (e as { download_url?: string | null }).download_url)
    expect(withDownload.length).toBeGreaterThan(0)
    for (const e of withDownload) {
      expect(e.sha256, e.id).toMatch(/^[0-9a-f]{64}$/)
      expect(e.protocol, e.id).toBeGreaterThanOrEqual(1)
    }
  })
})
