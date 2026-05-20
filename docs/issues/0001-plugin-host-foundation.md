# 0001 — Plugin Host foundation + mangadex migration

**Status**: done
**Labels**: enhancement, ready-for-agent
**Blocked by**: none

## What to build

Build the plugin-host container and prove the full plugin-host stack end-to-end with mangadex as the first migrated bundle.

**Plugin Host container** (`plugin-host/`):
- Node.js/Express server that loads plugin bundles as modules from a `bundles/` directory
- Routes `GET /<plugin-id>/search`, `GET /<plugin-id>/manga/:id/chapters`, `GET /<plugin-id>/chapter/:id/pages`, `GET /<plugin-id>/trending`, `GET /<plugin-id>/manga/:id/meta`, `GET /<plugin-id>/cover` to the corresponding bundle's exported functions
- `GET /plugins` meta-endpoint returns array of `{ id, name, default_explicit, content_types }` for all loaded bundles
- Injects `PluginContext` into each bundle via `init(ctx)` on load — context shape: `{ getBrowser: () => Promise<Browser>, logger }` (stub `getBrowser` returning null for now; wired in 0002)
- Hot-reloads when a new `.js` file appears in `bundles/`

**mangadex migration**:
- Strip Express server from `plugins/mangadex/src/index.ts`
- Keep scraping logic in `plugins/mangadex/src/mangadex.ts` unchanged
- New `index.ts` exports: `info` (static object), `init(ctx)`, `search(q)`, `chapters(id, langs)`, `pages(id)`, `trending()`, `meta(id, langs)`
- Bundle compiles to a single `mangadex.js` via esbuild

**Rust server update**:
- When a source URL responds to `GET /plugins` with an array, register each entry as a separate source using `<base_url>/<plugin_id>` as the effective URL
- Existing single-source `/info` registration continues to work unchanged

**docker-compose**:
- Add `plugin-host` service alongside existing 6 plugin containers (those removed in 0003)
- `PLUGIN_URLS` includes `http://plugin-host:4000`

## Acceptance criteria

- [ ] `GET http://plugin-host:4000/plugins` returns JSON array including mangadex entry
- [ ] Discover search returns mangadex results routed through plugin-host
- [ ] Manga detail + chapter list works for a mangadex title via plugin-host
- [ ] Rust server registers plugin-host sources without crashing alongside standalone containers
- [ ] `docker compose up` starts cleanly with plugin-host present
