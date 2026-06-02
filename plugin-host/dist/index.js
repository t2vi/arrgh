"use strict";
var __importDefault = (this && this.__importDefault) || function (mod) {
    return (mod && mod.__esModule) ? mod : { "default": mod };
};
Object.defineProperty(exports, "__esModule", { value: true });
exports.createApp = createApp;
const express_1 = __importDefault(require("express"));
const fs_1 = __importDefault(require("fs"));
const path_1 = __importDefault(require("path"));
const playwright_core_1 = require("playwright-core");
const PORT = parseInt(process.env.PORT ?? '4000', 10);
const BUNDLES_DIR = process.env.BUNDLES_DIR ?? path_1.default.join(__dirname, '..', 'bundles');
const COMMUNITY_BUNDLES_DIR = process.env.COMMUNITY_BUNDLES_DIR ?? path_1.default.join(__dirname, '..', 'community-bundles');
const LANGS = (process.env.LANGUAGES ?? 'en').split(',').map((s) => s.trim()).filter(Boolean);
const CLOAKBROWSER_WS_URL = process.env.CLOAKBROWSER_WS_URL ?? '';
// ── CloakBrowser connection ───────────────────────────────────────────────────
let browser = null;
async function getBrowser() {
    if (browser?.isConnected())
        return browser;
    if (!CLOAKBROWSER_WS_URL) {
        throw new Error('CLOAKBROWSER_WS_URL is not set — CF-dependent plugins will not work');
    }
    console.log('[plugin-host] connecting to CloakBrowser…');
    // Fetch WS URL from /json/version and rewrite the internal Docker hostname to localhost,
    // so dev setups (plugin-host on host OS, CloakBrowser in Docker) work without DNS for container names.
    const versionRes = await fetch(`${CLOAKBROWSER_WS_URL}/json/version`);
    const { webSocketDebuggerUrl } = await versionRes.json();
    const port = new URL(CLOAKBROWSER_WS_URL).port || '80';
    const wsUrl = webSocketDebuggerUrl.replace(/^ws:\/\/[^/]+/, `ws://localhost:${port}`);
    browser = await playwright_core_1.chromium.connectOverCDP(wsUrl);
    browser.on('disconnected', () => {
        console.warn('[plugin-host] CloakBrowser disconnected — will reconnect on next request');
        browser = null;
    });
    console.log('[plugin-host] connected to CloakBrowser');
    return browser;
}
// ── Registry ──────────────────────────────────────────────────────────────────
const plugins = new Map();
const communityIds = new Set();
const ctx = {
    getBrowser,
    logger: console,
};
async function loadBundle(registry, communitySet, file, isCommunity = false) {
    const abs = path_1.default.resolve(file);
    try {
        delete require.cache[abs];
        // eslint-disable-next-line @typescript-eslint/no-require-imports
        const bundle = require(abs);
        if (bundle.init)
            await bundle.init(ctx);
        registry.set(bundle.info.id, bundle);
        if (isCommunity)
            communitySet.add(bundle.info.id);
        console.log(`[plugin-host] loaded: ${bundle.info.id} (${bundle.info.name})${isCommunity ? ' [community]' : ''}`);
    }
    catch (e) {
        console.error(`[plugin-host] failed to load ${path_1.default.basename(file)}:`, e);
    }
}
async function loadAll() {
    if (fs_1.default.existsSync(BUNDLES_DIR)) {
        const files = fs_1.default.readdirSync(BUNDLES_DIR).filter((f) => f.endsWith('.js'));
        for (const f of files)
            await loadBundle(plugins, communityIds, path_1.default.join(BUNDLES_DIR, f), false);
    }
    else {
        console.warn(`[plugin-host] bundles dir not found: ${BUNDLES_DIR}`);
    }
    if (fs_1.default.existsSync(COMMUNITY_BUNDLES_DIR)) {
        const files = fs_1.default.readdirSync(COMMUNITY_BUNDLES_DIR).filter((f) => f.endsWith('.js'));
        for (const f of files)
            await loadBundle(plugins, communityIds, path_1.default.join(COMMUNITY_BUNDLES_DIR, f), true);
    }
}
function watchBundles() {
    for (const [dir, isCommunity] of [[BUNDLES_DIR, false], [COMMUNITY_BUNDLES_DIR, true]]) {
        if (!fs_1.default.existsSync(dir))
            continue;
        fs_1.default.watch(dir, (_event, filename) => {
            if (filename && filename.endsWith('.js')) {
                loadBundle(plugins, communityIds, path_1.default.join(dir, filename), isCommunity).catch(console.error);
            }
        });
    }
    console.log(`[plugin-host] watching bundle directories`);
}
// ── App factory ───────────────────────────────────────────────────────────────
function createApp(registry, communityPluginIds = new Set()) {
    const app = (0, express_1.default)();
    app.use(express_1.default.json());
    function getPlugin(id, res) {
        const p = registry.get(id);
        if (!p) {
            res.status(404).json({ error: `plugin not found: ${id}` });
            return null;
        }
        return p;
    }
    app.get('/plugins', (_req, res) => {
        res.json(Array.from(registry.values()).map((p) => ({
            ...p.info,
            is_community: communityPluginIds.has(p.info.id),
        })));
    });
    app.get('/:plugin/info', (req, res) => {
        const p = registry.get(req.params.plugin);
        if (!p)
            return void res.status(404).json({ error: `plugin not found: ${req.params.plugin}` });
        res.json({ ...p.info, is_community: communityPluginIds.has(p.info.id) });
    });
    app.post('/plugins/install', async (req, res) => {
        const url = String(req.body?.url ?? '').trim();
        if (!url)
            return void res.status(400).json({ error: 'url required' });
        fs_1.default.mkdirSync(COMMUNITY_BUNDLES_DIR, { recursive: true });
        let bundleCode;
        try {
            const resp = await fetch(url);
            if (!resp.ok)
                throw new Error(`download failed: ${resp.status}`);
            bundleCode = await resp.text();
        }
        catch (e) {
            return void res.status(502).json({ error: String(e) });
        }
        const filename = path_1.default.basename(new URL(url).pathname);
        if (!filename.endsWith('.js')) {
            return void res.status(400).json({ error: 'download_url must end with .js' });
        }
        const dest = path_1.default.join(COMMUNITY_BUNDLES_DIR, filename);
        fs_1.default.writeFileSync(dest, bundleCode, 'utf-8');
        await loadBundle(registry, communityPluginIds, dest, true);
        res.status(201).json({ ok: true });
    });
    app.delete('/plugins/:id', (req, res) => {
        const id = req.params.id;
        if (!communityPluginIds.has(id)) {
            return void res.status(403).json({ error: 'cannot delete a bundled default plugin' });
        }
        registry.delete(id);
        communityPluginIds.delete(id);
        if (fs_1.default.existsSync(COMMUNITY_BUNDLES_DIR)) {
            const file = fs_1.default.readdirSync(COMMUNITY_BUNDLES_DIR).find((f) => f.startsWith(id));
            if (file) {
                try {
                    fs_1.default.unlinkSync(path_1.default.join(COMMUNITY_BUNDLES_DIR, file));
                }
                catch { /* ignore */ }
            }
        }
        res.status(204).send();
    });
    app.get('/:plugin/search', async (req, res) => {
        const p = getPlugin(req.params.plugin, res);
        if (!p)
            return;
        const q = String(req.query['q'] ?? '').trim();
        if (!q)
            return void res.json([]);
        try {
            res.json(await p.search(q));
        }
        catch (e) {
            console.error(`[${req.params.plugin}] search error:`, e);
            res.status(502).json({ error: String(e) });
        }
    });
    app.get('/:plugin/trending', async (req, res) => {
        const p = getPlugin(req.params.plugin, res);
        if (!p)
            return;
        if (!p.trending)
            return void res.status(404).json({ error: 'trending not supported' });
        try {
            res.json(await p.trending());
        }
        catch (e) {
            console.error(`[${req.params.plugin}] trending error:`, e);
            res.status(502).json({ error: String(e) });
        }
    });
    app.get('/:plugin/manga/:id/meta', async (req, res) => {
        const p = getPlugin(req.params.plugin, res);
        if (!p)
            return;
        if (!p.meta)
            return void res.status(404).json({ error: 'meta not supported' });
        try {
            res.json(await p.meta(decodeURIComponent(req.params.id), LANGS));
        }
        catch (e) {
            console.error(`[${req.params.plugin}] meta error:`, e);
            res.status(502).json({ error: String(e) });
        }
    });
    app.get('/:plugin/manga/:id/chapters', async (req, res) => {
        const p = getPlugin(req.params.plugin, res);
        if (!p)
            return;
        try {
            res.json(await p.chapters(decodeURIComponent(req.params.id), LANGS));
        }
        catch (e) {
            console.error(`[${req.params.plugin}] chapters error:`, e);
            res.status(502).json({ error: String(e) });
        }
    });
    app.get('/:plugin/chapter/:id/pages', async (req, res) => {
        const p = getPlugin(req.params.plugin, res);
        if (!p)
            return;
        if (!p.pages)
            return void res.status(404).json({ error: 'pages not supported by this plugin' });
        try {
            res.json(await p.pages(decodeURIComponent(req.params.id)));
        }
        catch (e) {
            console.error(`[${req.params.plugin}] pages error:`, e);
            res.status(502).json({ error: String(e) });
        }
    });
    app.get('/:plugin/chapter/:id/text', async (req, res) => {
        const p = getPlugin(req.params.plugin, res);
        if (!p)
            return;
        if (!p.chapterText)
            return void res.status(404).json({ error: 'chapter text not supported by this plugin' });
        try {
            res.type('text/plain').send(await p.chapterText(decodeURIComponent(req.params.id)));
        }
        catch (e) {
            console.error(`[${req.params.plugin}] chapter text error:`, e);
            res.status(502).json({ error: String(e) });
        }
    });
    app.get('/:plugin/cover', async (req, res) => {
        const p = getPlugin(req.params.plugin, res);
        if (!p)
            return;
        if (!p.cover)
            return void res.status(501).json({ error: 'cover proxy not implemented' });
        const url = String(req.query['url'] ?? '');
        if (!url)
            return void res.status(400).json({ error: 'url query param required' });
        try {
            const buf = await p.cover(url);
            res.set('Content-Type', 'image/jpeg').send(buf);
        }
        catch (e) {
            res.status(502).json({ error: String(e) });
        }
    });
    return app;
}
// ── Boot ──────────────────────────────────────────────────────────────────────
loadAll().then(() => {
    watchBundles();
    const app = createApp(plugins, communityIds);
    app.listen(PORT, () => {
        console.log(`[plugin-host] listening on :${PORT} — ${plugins.size} plugin(s) loaded`);
        console.log(`[plugin-host] languages: ${LANGS.join(', ')}`);
        console.log(`[plugin-host] cloakbrowser: ${CLOAKBROWSER_WS_URL || 'not configured (CF plugins will fail)'}`);
    });
}).catch((e) => {
    console.error('[plugin-host] boot failed:', e);
    process.exit(1);
});
