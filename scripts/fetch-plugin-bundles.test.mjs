// node --test scripts/fetch-plugin-bundles.test.mjs — no network, tests only the pure filter.
// The actual fetch/checksum/version-verify logic is exercised for real by the Docker build and
// by CI's `npm run fetch:plugins` step (spec 031 phase D) — network-dependent, not mocked here.
import { test } from 'node:test'
import assert from 'node:assert/strict'
import { bundledEntries } from './fetch-plugin-bundles.mjs'

test('bundledEntries: filters to only bundled: true entries', () => {
  const index = [
    { id: 'a', bundled: true },
    { id: 'b', bundled: false },
    { id: 'c' },
  ]
  assert.deepEqual(bundledEntries(index).map((e) => e.id), ['a'])
})

test('bundledEntries: real plugin-index.json only lists bundled plugins as bundled', async () => {
  const { readFileSync } = await import('node:fs')
  const index = JSON.parse(readFileSync(new URL('../plugin-index/index.json', import.meta.url), 'utf8'))
  const entries = bundledEntries(index)
  assert.ok(entries.length > 0)
  assert.ok(!entries.some((e) => e.id === 'fixture'))
  for (const e of entries) {
    assert.equal(typeof e.download_url, 'string')
    assert.match(e.sha256, /^[0-9a-f]{64}$/)
  }
})

// spec: 031/FR-019
test('stalePins: lists only entries whose pin is behind the latest release', async () => {
  const { stalePins } = await import('./fetch-plugin-bundles.mjs')
  const entries = [
    { id: 'a', version: '1.0.0' },
    { id: 'b', version: '1.0.1' },
    { id: 'c', version: '1.0.0' },
  ]
  const latest = { a: { version: '1.0.1' }, b: { version: '1.0.1' }, c: { version: '1.0.0' } }
  assert.deepEqual(stalePins(entries, latest), [{ id: 'a', pinned: '1.0.0', latest: '1.0.1' }])
})

// spec: 031/FR-019
test('repoOf: derives owner/repo from a GitHub release download_url', async () => {
  const { repoOf } = await import('./fetch-plugin-bundles.mjs')
  assert.equal(
    repoOf({ download_url: 'https://github.com/t2vi/arrgh-plugin-mangadex/releases/download/v1.0.0/mangadex.js' }),
    't2vi/arrgh-plugin-mangadex',
  )
})

// spec: 031/FR-019
test('repin: rewrites only the stale entry\'s version/url/sha256, keeps formatting', async () => {
  const { repin } = await import('./fetch-plugin-bundles.mjs')
  const text = `[
  {
    "id": "a",
    "version": "1.0.0",
    "download_url": "https://github.com/o/arrgh-plugin-a/releases/download/v1.0.0/a.js",
    "sha256": "${'1'.repeat(64)}",
    "content_types": ["manga"]
  },
  {
    "id": "b",
    "version": "1.0.0",
    "download_url": "https://github.com/o/arrgh-plugin-b/releases/download/v1.0.0/b.js",
    "sha256": "${'2'.repeat(64)}"
  }
]
`
  const out = repin(text, 'a', {
    version: '1.0.1',
    download_url: 'https://github.com/o/arrgh-plugin-a/releases/download/v1.0.1/a.js',
    sha256: '3'.repeat(64),
  })
  const want = text
    .replace('"version": "1.0.0",\n    "download_url": "https://github.com/o/arrgh-plugin-a/releases/download/v1.0.0/a.js"',
      '"version": "1.0.1",\n    "download_url": "https://github.com/o/arrgh-plugin-a/releases/download/v1.0.1/a.js"')
    .replace('1'.repeat(64), '3'.repeat(64))
  assert.equal(out, want)
  assert.match(out, /"id": "b",\n    "version": "1.0.0"/) // b untouched
})
