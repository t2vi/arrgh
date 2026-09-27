#!/usr/bin/env node
// Fetch every bundled plugin's published release asset from plugin-index/index.json,
// verify its sha256, and write it to --out (default plugin-host/bundles/). Fails loudly on
// any missing url/sha256, download error, or checksum mismatch (spec 031 FR-016).
// Skips a file already on disk whose sha256 already matches — no needless refetch.
import { createHash } from 'node:crypto'
import { mkdirSync, readFileSync, writeFileSync, existsSync } from 'node:fs'
import path from 'node:path'
import { parseArgs } from 'node:util'
import { fileURLToPath } from 'node:url'
import { createRequire } from 'node:module'

const require = createRequire(import.meta.url)

const repoRoot = path.join(path.dirname(fileURLToPath(import.meta.url)), '..')

/** The plugin-index entries this script is responsible for fetching. */
export function bundledEntries(indexJson) {
  return indexJson.filter((e) => e.bundled)
}

function sha256(buf) {
  return createHash('sha256').update(buf).digest('hex')
}

function checkVersion(entry, dest) {
  delete require.cache[dest]
  const { info } = require(dest)
  if (info?.version !== entry.version) {
    throw new Error(
      `${entry.id}: bundle reports info.version "${info?.version}" but plugin-index/index.json says "${entry.version}"`,
    )
  }
}

async function fetchOne(entry, outDir) {
  const { id, download_url: url, sha256: expected } = entry
  if (!url || !expected) throw new Error(`${id}: missing download_url or sha256 in plugin-index/index.json`)

  const dest = path.join(outDir, `${id}.js`)
  if (existsSync(dest) && sha256(readFileSync(dest)) === expected) {
    console.log(`[fetch-plugin-bundles] ${id}: up to date`)
    checkVersion(entry, dest)
    return
  }

  console.log(`[fetch-plugin-bundles] ${id}: fetching ${url}`)
  const res = await fetch(url)
  if (!res.ok) throw new Error(`${id}: download failed: HTTP ${res.status} (${url})`)
  const buf = Buffer.from(await res.arrayBuffer())

  const actual = sha256(buf)
  if (actual !== expected) {
    throw new Error(`${id}: checksum mismatch — expected ${expected}, got ${actual} (${url})`)
  }

  writeFileSync(dest, buf)
  checkVersion(entry, dest)
  console.log(`[fetch-plugin-bundles] ${id}: verified, wrote ${dest}`)
}

async function main() {
  const { values } = parseArgs({
    options: {
      index: { type: 'string', default: path.join(repoRoot, 'plugin-index/index.json') },
      out: { type: 'string', default: path.join(repoRoot, 'plugin-host/bundles') },
    },
  })

  const indexJson = JSON.parse(readFileSync(values.index, 'utf8'))
  const entries = bundledEntries(indexJson)
  // Absolute: require()'s relative-specifier resolution is anchored to this script's own
  // directory, not process.cwd() — a relative --out would silently resolve to the wrong place.
  const outDir = path.resolve(values.out)
  mkdirSync(outDir, { recursive: true })

  for (const entry of entries) {
    await fetchOne(entry, outDir)
  }
  console.log(`[fetch-plugin-bundles] ${entries.length} bundled plugin(s) verified in ${outDir}`)
}

if (import.meta.url === `file://${process.argv[1]}`) {
  main().catch((e) => {
    console.error(`[fetch-plugin-bundles] ${e instanceof Error ? e.message : e}`)
    process.exit(1)
  })
}
