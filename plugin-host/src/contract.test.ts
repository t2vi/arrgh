// Contract tests for plugin-index/index.json itself, plus generic cross-checks against
// whatever plugin dirs remain bundled (frozen fallback copies — see plugins/<id>/README.md).
// Per-plugin fixture/behavior/contract tests now live in each plugin's own repo
// (t2vi/arrgh-plugin-<id>, spec 031 phase C) — no HTTP calls here.

import { describe, it, expect, beforeAll } from 'vitest'
import { readFileSync, readdirSync, existsSync } from 'node:fs'

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
})

// ── Production plugin set (spec 020) ─────────────────────────────────────────
// One definition, three places: the plugin-host image, `npm run build:plugins`
// (also what scripts/sync-plugins.sh builds for dev), and the bundled index.

describe('production plugin set', () => {
  const root = new URL('../../', import.meta.url)
  const read = (p: string) => readFileSync(new URL(p, root), 'utf8')

  const dockerfile = [...read('plugin-host/Dockerfile').matchAll(
    /COPY --from=bundles \/build\/plugins\/([^/]+)\/bundles\/\1\.js/g,
  )].map((m) => m[1]).sort()
  const buildScript = JSON.parse(read('package.json')).scripts['build:plugins'] as string
  const built = [...buildScript.matchAll(/-w plugins\/(\S+)/g)].map((m) => m[1]).sort()
  const index = (JSON.parse(read('plugin-index/index.json')) as { id: string; bundled?: boolean }[])
    .filter((p) => p.bundled).map((p) => p.id).sort()

  it('Dockerfile, build:plugins and index bundled list the same plugins', () => {
    expect(dockerfile.length).toBeGreaterThan(0)
    expect(built).toEqual(dockerfile)
    expect(index).toEqual(dockerfile)
  })

  it('never ships the e2e fixture plugin', () => {
    expect(dockerfile).not.toContain('fixture')
  })
})

// spec: 031/FR-010, 031/FR-001
describe('plugins report their own version (spec 031)', () => {
  const index = indexJson as { id: string; version: string; bundled?: boolean; sha256?: string | null; protocol?: number }[]
  const pluginsDir = new URL('../../plugins/', import.meta.url)
  const ids = readdirSync(pluginsDir).filter((d) => existsSync(new URL(`${d}/src/index.ts`, pluginsDir)))
  let infos: { id: string; version?: string }[] = []
  beforeAll(async () => {
    infos = await Promise.all(ids.map(async (d) => (await import(`../../plugins/${d}/src/index.ts`)).info))
  })

  it('every bundled plugin is checked', () => {
    expect(infos.map((i) => i.id).sort()).toEqual(index.filter((e) => e.bundled).map((e) => e.id).sort())
  })

  it.each(index.filter((e) => e.bundled).map((e) => [e.id, e] as const))('%s: info.version equals the index version', (id, entry) => {
    expect(infos.find((i) => i.id === id)?.version).toBe(entry.version)
  })

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
