#!/usr/bin/env node
// Fetch every bundled plugin's published release asset from plugin-index/index.json,
// verify its sha256, and write it to --out (default plugin-host/bundles/). Fails loudly on
// any missing url/sha256, download error, or checksum mismatch (spec 031 FR-016).
// Skips a file already on disk whose sha256 already matches — no needless refetch.
// --check-latest / --bump: compare (or re-pin) every bundled pin against its repo's latest
// GitHub release instead of fetching (spec 031 FR-019, #262).
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

/** `owner/repo` of the GitHub repository an entry's release asset lives in. */
export function repoOf(entry) {
  return new URL(entry.download_url).pathname.split('/').slice(1, 3).join('/')
}

/** Bundled entries whose pin differs from their repo's latest release (spec 031/FR-019). */
export function stalePins(entries, latest) {
  // ponytail: "differs" not semver-"behind" — a pin newer than the latest release can't happen
  return entries
    .filter((e) => latest[e.id] && latest[e.id].version !== e.version)
    .map((e) => ({ id: e.id, pinned: e.version, latest: latest[e.id].version }))
}

/** Rewrite one entry's version/download_url/sha256 in the raw index text, keeping its formatting. */
export function repin(text, id, { version, download_url, sha256 }) {
  const start = text.indexOf(`"id": "${id}"`)
  if (start < 0) throw new Error(`${id}: not found in plugin-index/index.json`)
  const end = text.indexOf('}', start)
  const block = text
    .slice(start, end)
    .replace(/"version": "[^"]*"/, `"version": "${version}"`)
    .replace(/"download_url": "[^"]*"/, `"download_url": "${download_url}"`)
    .replace(/"sha256": "[^"]*"/, `"sha256": "${sha256}"`)
  return text.slice(0, start) + block + text.slice(end)
}

async function latestRelease(entry) {
  const repo = repoOf(entry)
  const headers = { accept: 'application/vnd.github+json' }
  const token = process.env.GITHUB_TOKEN || process.env.GH_TOKEN
  if (token) headers.authorization = `Bearer ${token}`
  const res = await fetch(`https://api.github.com/repos/${repo}/releases/latest`, { headers })
  if (!res.ok) {
    const hint = [403, 429].includes(res.status) && !token ? ' — rate-limited; run with GITHUB_TOKEN=$(gh auth token)' : ''
    throw new Error(`${entry.id}: latest-release lookup failed: HTTP ${res.status} (${repo})${hint}`)
  }
  const rel = await res.json()
  const asset = rel.assets?.find((a) => a.name === `${entry.id}.js`)
  if (!asset) throw new Error(`${entry.id}: latest release ${rel.tag_name} has no ${entry.id}.js asset (${repo})`)
  return { version: rel.tag_name.replace(/^v/, ''), download_url: asset.browser_download_url }
}

/** --check-latest: fail listing stale pins. --bump: re-pin them in place (spec 031/FR-019). */
async function checkLatest(indexPath, bump) {
  const text = readFileSync(indexPath, 'utf8')
  const entries = bundledEntries(JSON.parse(text))
  const latest = {}
  for (const e of entries) latest[e.id] = await latestRelease(e)
  const stale = stalePins(entries, latest)
  if (!stale.length) {
    console.log(`[fetch-plugin-bundles] all ${entries.length} bundled pins match their latest release`)
    return
  }
  if (!bump) {
    for (const s of stale) console.error(`[fetch-plugin-bundles] ${s.id}: pinned ${s.pinned}, latest ${s.latest}`)
    throw new Error(`${stale.length} stale pin(s) — run: node scripts/fetch-plugin-bundles.mjs --bump`)
  }
  let out = text
  for (const s of stale) {
    const { download_url } = latest[s.id]
    const res = await fetch(download_url)
    if (!res.ok) throw new Error(`${s.id}: download failed: HTTP ${res.status} (${download_url})`)
    const sum = sha256(Buffer.from(await res.arrayBuffer()))
    out = repin(out, s.id, { version: s.latest, download_url, sha256: sum })
    console.log(`[fetch-plugin-bundles] ${s.id}: re-pinned ${s.pinned} → ${s.latest}`)
  }
  writeFileSync(indexPath, out)
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
      'check-latest': { type: 'boolean', default: false },
      bump: { type: 'boolean', default: false },
    },
  })
  if (values['check-latest'] || values.bump) return checkLatest(values.index, values.bump)

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
