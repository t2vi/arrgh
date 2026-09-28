import express from 'express'
import { createHash, randomUUID } from 'crypto'
import fs from 'fs'
import path from 'path'
import { chromium } from 'playwright-core'
import type { Browser } from 'playwright-core'

const PORT = parseInt(process.env.PORT ?? '4000', 10)
const BUNDLES_DIR = process.env.BUNDLES_DIR ?? path.join(__dirname, '..', 'bundles')
const COMMUNITY_BUNDLES_DIR = process.env.COMMUNITY_BUNDLES_DIR ?? path.join(__dirname, '..', 'community-bundles')
const LANGS = (process.env.LANGUAGES ?? 'en').split(',').map((s) => s.trim()).filter(Boolean)
const CLOAKBROWSER_WS_URL = process.env.CLOAKBROWSER_WS_URL ?? ''
// Upper bound on one plugin call. Generous: a long novel's chapter list can page for a while.
const PLUGIN_CALL_TIMEOUT_MS = parseInt(process.env.PLUGIN_CALL_TIMEOUT_MS ?? '180000', 10)

class PluginTimeoutError extends Error {}

// ponytail: stops waiting, doesn't cancel — the plugin API has no AbortSignal, so a hung
// fetch/CDP call keeps running in the background. Thread a signal through if that leaks.
function withTimeout<T>(call: Promise<T>, ms: number, what: string): Promise<T> {
  let timer: NodeJS.Timeout | undefined
  const deadline = new Promise<never>((_, reject) => {
    timer = setTimeout(() => reject(new PluginTimeoutError(`${what} timed out after ${ms}ms`)), ms)
  })
  return Promise.race([call, deadline]).finally(() => clearTimeout(timer))
}

const errorStatus = (e: unknown) => (e instanceof PluginTimeoutError ? 504 : 502)

// ── CloakBrowser connection ───────────────────────────────────────────────────

// Rewrite the host portion of a CDP WebSocket URL reported by /json/version to the
// host from the configured CLOAKBROWSER_WS_URL. CloakBrowser self-reports 0.0.0.0 or
// its internal container hostname — neither is reachable from outside. Using the
// configured URL's host works for both local dev (localhost:3001) and Docker-to-Docker
// (cloakbrowser:3000) without any special-casing.
export function rewriteCdpHost(cdpWsUrl: string, configUrl: string): string {
  const { host } = new URL(configUrl)
  return cdpWsUrl.replace(/^ws:\/\/[^/]+/, `ws://${host}`)
}

let browser: Browser | null = null

/** Test seam — clears the cached connection so the next `getBrowser()` reconnects. */
export function resetBrowserForTest(): void {
  browser = null
}

export async function getBrowser(): Promise<Browser> {
  if (browser?.isConnected()) return browser
  if (!CLOAKBROWSER_WS_URL) {
    throw new Error('CLOAKBROWSER_WS_URL is not set — CF-dependent plugins will not work')
  }
  console.log('[plugin-host] connecting to CloakBrowser…')
  // Fetch WS URL from /json/version and rewrite the host to match CLOAKBROWSER_WS_URL.
  // CloakBrowser reports its own internal hostname in the WS URL (e.g. 0.0.0.0), which is
  // unreachable from outside. Use the host from CLOAKBROWSER_WS_URL instead — this works for
  // both local dev (localhost:3001) and Docker-to-Docker (cloakbrowser:3000).
  const versionRes = await fetch(`${CLOAKBROWSER_WS_URL}/json/version`)
  const { webSocketDebuggerUrl } = await versionRes.json() as { webSocketDebuggerUrl: string }
  const wsUrl = rewriteCdpHost(webSocketDebuggerUrl, CLOAKBROWSER_WS_URL)
  browser = await chromium.connectOverCDP(wsUrl)
  browser.on('disconnected', () => {
    console.warn('[plugin-host] CloakBrowser disconnected — will reconnect on next request')
    browser = null
  })
  console.log('[plugin-host] connected to CloakBrowser')
  return browser
}

// ── Plugin protocol types ─────────────────────────────────────────────────────

export interface PluginInfo {
  id: string
  name: string
  /** The bundle's own version (spec 031 FR-010); absent on older builds → shown as unknown. */
  version?: string
  default_explicit: boolean
  content_types: string[]
  is_community?: boolean
}

export interface PluginContext {
  getBrowser: () => Promise<Browser>
  logger: typeof console
}

export interface PluginBundle {
  info: PluginInfo
  init?: (ctx: PluginContext) => void | Promise<void>
  search: (q: string) => Promise<unknown[]>
  chapters: (id: string, langs: string[]) => Promise<unknown[]>
  pages?: (id: string) => Promise<unknown[]>
  trending?: () => Promise<unknown[]>
  meta?: (id: string, langs: string[]) => Promise<unknown>
  cover?: (url: string) => Promise<Buffer>
  chapterText?: (id: string) => Promise<string>
}

// ── Registry ──────────────────────────────────────────────────────────────────

/** Plugin protocol this host implements. A catalog entry needing more can't be installed (spec 031 FR-006). */
export const PLUGIN_PROTOCOL = 1

export const ORIGIN_BUNDLED = 'bundled'
export const ORIGIN_DOWNLOADED = 'downloaded'
export type Origin = typeof ORIGIN_BUNDLED | typeof ORIGIN_DOWNLOADED

export interface Loaded { bundle: PluginBundle; file?: string }
export interface Slots { bundled?: Loaded; downloaded?: Loaded }

/** Numeric dot-separated compare; a pre-release suffix is ignored. ponytail: not full semver. */
export function compareVersions(a: string, b: string): number {
  const parts = (v: string) => v.split('-')[0].split('.').map((n) => parseInt(n, 10) || 0)
  const [pa, pb] = [parts(a), parts(b)]
  for (let i = 0; i < Math.max(pa.length, pb.length); i++) {
    const d = (pa[i] ?? 0) - (pb[i] ?? 0)
    if (d !== 0) return d
  }
  return 0
}

/** Downloaded wins, unless both versions are known and the bundled one is newer (spec 031 FR-008). */
export function pickActive({ bundled, downloaded }: Slots): Loaded | undefined {
  if (!downloaded || !bundled) return downloaded ?? bundled
  const [bv, dv] = [bundled.bundle.info.version, downloaded.bundle.info.version]
  return bv && dv && compareVersions(bv, dv) > 0 ? bundled : downloaded
}

/** Per id: a bundled and a downloaded slot; `active` is what the routes serve. */
export class PluginRegistry {
  readonly slots = new Map<string, Slots>()

  /** `active` may be a caller's map; its existing entries count as bundled, or downloaded when in `downloadedIds`. */
  constructor(readonly active: Map<string, PluginBundle> = new Map(), downloadedIds: Set<string> = new Set()) {
    for (const [id, bundle] of active) {
      this.slots.set(id, { [downloadedIds.has(id) ? ORIGIN_DOWNLOADED : ORIGIN_BUNDLED]: { bundle } })
    }
  }

  put(id: string, origin: Origin, loaded: Loaded | undefined): void {
    const slots = { ...this.slots.get(id), [origin]: loaded }
    const pick = pickActive(slots)
    if (!pick) {
      this.slots.delete(id)
      this.active.delete(id)
      return
    }
    this.slots.set(id, slots)
    this.active.set(id, pick.bundle)
  }

  origin(id: string): Origin | undefined {
    const slots = this.slots.get(id)
    if (!slots) return undefined
    return pickActive(slots) === slots.downloaded ? ORIGIN_DOWNLOADED : ORIGIN_BUNDLED
  }

  /** Watcher: a bundle file disappeared — drop whichever slot it filled. */
  removeFile(file: string): void {
    for (const [id, slots] of this.slots) {
      for (const origin of [ORIGIN_BUNDLED, ORIGIN_DOWNLOADED] as const) {
        if (slots[origin]?.file === file) {
          this.put(id, origin, undefined)
          console.log(`[plugin-host] unloaded ${origin}: ${id} (bundle removed)`)
        }
      }
    }
  }
}

const registry = new PluginRegistry()

const ctx: PluginContext = {
  getBrowser,
  logger: console,
}

/** require + init a bundle file; throws on any failure. */
async function requireBundle(file: string): Promise<PluginBundle> {
  delete require.cache[file]
  // eslint-disable-next-line @typescript-eslint/no-require-imports
  const bundle: PluginBundle = require(file)
  if (!bundle?.info?.id) throw new Error('bundle has no info.id')
  if (bundle.init) await bundle.init(ctx)
  return bundle
}

async function loadBundle(reg: PluginRegistry, file: string, origin: Origin): Promise<void> {
  const abs = path.resolve(file)
  try {
    const bundle = await requireBundle(abs)
    reg.put(bundle.info.id, origin, { bundle, file: abs })
    console.log(`[plugin-host] loaded ${origin}: ${bundle.info.id} ${bundle.info.version ?? '(version unknown)'}`)
  } catch (e) {
    console.error(`[plugin-host] failed to load ${path.basename(file)}:`, e)
  }
}

export async function loadDir(reg: PluginRegistry, dir: string, origin: Origin): Promise<void> {
  for (const f of fs.readdirSync(dir).filter((f) => f.endsWith('.js'))) await loadBundle(reg, path.join(dir, f), origin)
}

/** Watcher event: (re)load a changed bundle file, or unload it if the file was removed (spec 020). */
export async function onBundleChange(reg: PluginRegistry, file: string, origin: Origin): Promise<void> {
  const abs = path.resolve(file)
  if (fs.existsSync(abs)) return loadBundle(reg, abs, origin)
  reg.removeFile(abs)
}

async function loadAll(): Promise<void> {
  if (fs.existsSync(BUNDLES_DIR)) await loadDir(registry, BUNDLES_DIR, ORIGIN_BUNDLED)
  else console.warn(`[plugin-host] bundles dir not found: ${BUNDLES_DIR}`)
  if (fs.existsSync(COMMUNITY_BUNDLES_DIR)) await loadDir(registry, COMMUNITY_BUNDLES_DIR, ORIGIN_DOWNLOADED)
}

function watchBundles(): void {
  for (const [dir, origin] of [[BUNDLES_DIR, ORIGIN_BUNDLED], [COMMUNITY_BUNDLES_DIR, ORIGIN_DOWNLOADED]] as const) {
    if (!fs.existsSync(dir)) continue
    fs.watch(dir, (_event, filename) => {
      if (filename && filename.endsWith('.js')) {
        onBundleChange(registry, path.join(dir, filename), origin).catch(console.error)
      }
    })
  }
  console.log(`[plugin-host] watching bundle directories`)
}

// ── App factory ───────────────────────────────────────────────────────────────

export function createApp(
  plugins: PluginRegistry | Map<string, PluginBundle>,
  downloadedIds: Set<string> = new Set(),
  {
    callTimeoutMs = PLUGIN_CALL_TIMEOUT_MS,
    communityDir = COMMUNITY_BUNDLES_DIR,
  }: { callTimeoutMs?: number; communityDir?: string } = {},
): express.Application {
  const reg = plugins instanceof PluginRegistry ? plugins : new PluginRegistry(plugins, downloadedIds)
  const registry = reg.active
  const app = express()
  const call = <T>(req: express.Request, fn: string, work: Promise<T>) =>
    withTimeout(work, callTimeoutMs, `${req.params.plugin} ${fn}`)
  app.use(express.json())

  function getPlugin(id: string, res: express.Response): PluginBundle | null {
    const p = registry.get(id)
    if (!p) {
      res.status(404).json({ error: `plugin not found: ${id}` })
      return null
    }
    return p
  }

  const describe = (p: PluginBundle) => {
    const origin = reg.origin(p.info.id)
    return {
      ...p.info,
      version: p.info.version ?? null,
      origin,
      has_bundled: !!reg.slots.get(p.info.id)?.bundled,
      is_community: origin === ORIGIN_DOWNLOADED,
    }
  }

  app.get('/host', (_req, res) => {
    res.json({ protocol: PLUGIN_PROTOCOL })
  })

  app.get('/plugins', (_req, res) => {
    res.json(Array.from(registry.values()).map(describe))
  })

  app.get('/:plugin/info', (req, res) => {
    const p = registry.get(req.params.plugin)
    if (!p) return void res.status(404).json({ error: `plugin not found: ${req.params.plugin}` })
    res.json(describe(p))
  })

  // Install or update one plugin by id (spec 031 FR-005/006/012). Nothing is written or swapped
  // unless the download matches its sha256 and the bundle loads and reports the same id.
  app.post('/plugins/install', async (req, res) => {
    const id = String(req.body?.id ?? '').trim()
    const url = String(req.body?.url ?? '').trim()
    const expected = String(req.body?.sha256 ?? '').trim().toLowerCase()
    const protocol = Number(req.body?.protocol ?? 1)
    if (!id || !url) return void res.status(400).json({ error: 'id and url required' })
    if (!expected) return void res.status(400).json({ error: 'sha256 required' })
    let pathname: string
    try { pathname = new URL(url).pathname } catch { return void res.status(400).json({ error: 'invalid url' }) }
    if (!pathname.endsWith('.js')) return void res.status(400).json({ error: 'download_url must end with .js' })
    if (protocol > PLUGIN_PROTOCOL) {
      return void res.status(422).json({
        error: `needs plugin protocol ${protocol}; this *ARRgh supports ${PLUGIN_PROTOCOL} — upgrade *ARRgh first`,
      })
    }

    let code: Buffer
    try {
      const resp = await fetch(url)
      if (!resp.ok) throw new Error(`download failed: ${resp.status}`)
      code = Buffer.from(await resp.arrayBuffer())
    } catch (e) {
      return void res.status(502).json({ error: String(e) })
    }
    const actual = createHash('sha256').update(code).digest('hex')
    if (actual !== expected) {
      return void res.status(422).json({ error: `checksum mismatch: expected ${expected}, got ${actual}` })
    }

    // Unique temp name per request (not .js, so the watcher ignores it); rename is atomic,
    // so two concurrent updates never leave a half-written <id>.js.
    fs.mkdirSync(communityDir, { recursive: true })
    const tmp = path.join(communityDir, `.${id}.${randomUUID()}.tmp`)
    fs.writeFileSync(tmp, code)
    let bundle: PluginBundle
    try {
      bundle = await requireBundle(tmp)
      if (bundle.info.id !== id) throw new Error(`bundle reports id "${bundle.info.id}", expected "${id}"`)
    } catch (e) {
      fs.rmSync(tmp, { force: true })
      return void res.status(422).json({ error: `bundle failed to load: ${e instanceof Error ? e.message : e}` })
    } finally {
      delete require.cache[tmp]
    }

    const dest = path.join(communityDir, `${id}.js`)
    const previous = reg.slots.get(id)?.downloaded?.file
    fs.renameSync(tmp, dest)
    if (previous && previous !== dest) fs.rmSync(previous, { force: true }) // legacy URL-named file
    reg.put(id, ORIGIN_DOWNLOADED, { bundle, file: dest })
    console.log(`[plugin-host] installed ${id} ${bundle.info.version ?? '(version unknown)'}`)
    res.status(201).json({ id, version: bundle.info.version ?? null, origin: reg.origin(id) })
  })

  // Revert to the bundled version, or unload a downloaded-only plugin (spec 031 FR-009).
  app.delete('/plugins/:id', (req, res) => {
    const id = req.params.id
    const slots = reg.slots.get(id)
    if (!slots) return void res.status(404).json({ error: `plugin not found: ${id}` })
    if (!slots.downloaded) {
      return void res.status(403).json({ error: 'nothing downloaded to remove — this is the bundled version' })
    }
    if (slots.downloaded.file) fs.rmSync(slots.downloaded.file, { force: true })
    reg.put(id, ORIGIN_DOWNLOADED, undefined)
    const now = registry.get(id)
    if (!now) return void res.status(204).send()
    res.json({ id, version: now.info.version ?? null, origin: reg.origin(id) })
  })

  app.get('/:plugin/search', async (req, res) => {
    const p = getPlugin(req.params.plugin, res)
    if (!p) return
    const q = String(req.query['q'] ?? '').trim()
    if (!q) return void res.json([])
    try {
      res.json(await call(req, 'search', p.search(q)))
    } catch (e) {
      console.error(`[${req.params.plugin}] search error:`, e)
      res.status(errorStatus(e)).json({ error: String(e) })
    }
  })

  app.get('/:plugin/trending', async (req, res) => {
    const p = getPlugin(req.params.plugin, res)
    if (!p) return
    if (!p.trending) return void res.status(404).json({ error: 'trending not supported' })
    try {
      res.json(await call(req, 'trending', p.trending()))
    } catch (e) {
      console.error(`[${req.params.plugin}] trending error:`, e)
      res.status(errorStatus(e)).json({ error: String(e) })
    }
  })

  app.get('/:plugin/manga/:id/meta', async (req, res) => {
    const p = getPlugin(req.params.plugin, res)
    if (!p) return
    if (!p.meta) return void res.status(404).json({ error: 'meta not supported' })
    try {
      res.json(await call(req, 'meta', p.meta(decodeURIComponent(req.params.id), LANGS)))
    } catch (e) {
      console.error(`[${req.params.plugin}] meta error:`, e)
      res.status(errorStatus(e)).json({ error: String(e) })
    }
  })

  app.get('/:plugin/manga/:id/chapters', async (req, res) => {
    const p = getPlugin(req.params.plugin, res)
    if (!p) return
    try {
      res.json(await call(req, 'chapters', p.chapters(decodeURIComponent(req.params.id), LANGS)))
    } catch (e) {
      console.error(`[${req.params.plugin}] chapters error:`, e)
      res.status(errorStatus(e)).json({ error: String(e) })
    }
  })

  app.get('/:plugin/chapter/:id/pages', async (req, res) => {
    const p = getPlugin(req.params.plugin, res)
    if (!p) return
    if (!p.pages) return void res.status(404).json({ error: 'pages not supported by this plugin' })
    try {
      res.json(await call(req, 'pages', p.pages(decodeURIComponent(req.params.id))))
    } catch (e) {
      console.error(`[${req.params.plugin}] pages error:`, e)
      res.status(errorStatus(e)).json({ error: String(e) })
    }
  })

  app.get('/:plugin/chapter/:id/text', async (req, res) => {
    const p = getPlugin(req.params.plugin, res)
    if (!p) return
    if (!p.chapterText) return void res.status(404).json({ error: 'chapter text not supported by this plugin' })
    try {
      const text = await call(req, 'chapterText', p.chapterText(decodeURIComponent(req.params.id)))
      res.type('text/plain').send(text)
    } catch (e) {
      console.error(`[${req.params.plugin}] chapter text error:`, e)
      res.status(errorStatus(e)).json({ error: String(e) })
    }
  })

  app.get('/:plugin/cover', async (req, res) => {
    const p = getPlugin(req.params.plugin, res)
    if (!p) return
    if (!p.cover) return void res.status(501).json({ error: 'cover proxy not implemented' })
    const url = String(req.query['url'] ?? '')
    if (!url) return void res.status(400).json({ error: 'url query param required' })
    try {
      const buf = await call(req, 'cover', p.cover(url))
      res.set('Content-Type', 'image/jpeg').send(buf)
    } catch (e) {
      res.status(errorStatus(e)).json({ error: String(e) })
    }
  })

  return app
}

// ── Boot ──────────────────────────────────────────────────────────────────────

// Tests import createApp from this module; don't also boot a real host
// (it would load real bundles and collide with a running dev host on :PORT).
if (!process.env.VITEST) loadAll().then(() => {
  watchBundles()
  const app = createApp(registry)
  app.listen(PORT, () => {
    console.log(`[plugin-host] listening on :${PORT} — ${registry.active.size} plugin(s) loaded`)
    console.log(`[plugin-host] languages: ${LANGS.join(', ')}`)
    console.log(`[plugin-host] cloakbrowser: ${CLOAKBROWSER_WS_URL || 'not configured (CF plugins will fail)'}`)
  })
}).catch((e) => {
  console.error('[plugin-host] boot failed:', e)
  process.exit(1)
})
