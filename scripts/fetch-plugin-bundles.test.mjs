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
