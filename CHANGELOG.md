# Changelog

All notable changes to *ARRgh are documented here.

---

## [0.0.7] v0.1.0

### Breaking changes

- **All plugins run in a single `plugin-host` container.** Individual per-source containers and ports (4000–4005) are removed. Update your `docker-compose.yml` to use `ghcr.io/t2vi/plugin-host:latest`.
- **FlareSolverr removed.** Replaced by `cloakbrowser` sidecar. Remove any `flaresolverr` service from your compose file.
- **`PLUGIN_URLS` default** — now `http://plugin-host:4000` (single endpoint for all sources).

### Features

- **Download progress** — live percentage bar per chapter in the Downloads queue and manga detail page; updates per-page during download
- **Plugin browse + install UI** — Settings → Sources → Browse shows all available plugins from the bundled index; install community plugins without restarting
- **Bundled plugin index** — `plugin-index/index.json` ships inside the arrgh image at `/app/plugin-index.json`; works air-gapped; override with `PLUGIN_INDEX_URL`

### Infrastructure

- **CloakBrowser replaces FlareSolverr** — stealth Chromium with 49 source-level C++ patches; bypasses Cloudflare Turnstile; accessed via CDP WebSocket. We run a custom wrapper (`docker/cloakbrowser-server/`: Python `serve.py` + nginx) on top of `cloakhq/cloakbrowser`.
- **Comick source** — migrated from expired `api.comick.fun` to `api.comick.dev`; CF-protected, routes through CloakBrowser
- **CI** — added `plugins` job (build all plugin bundles + plugin-host TypeScript); removed stale Flutter job
- **Docker workflow** — now builds and pushes three images: `arrgh`, `plugin-host`, `cloakbrowser-server`; cloakbrowser-server is amd64-only

### Bug fixes

- **Downloads saved to volume** — files were written to `/downloads` (ephemeral container layer) instead of `/data/downloads` (volume). Migration `0023_fix_download_dir` corrects the stored setting. Chapters downloaded before this release need to be re-queued.
- **Downloaded chapter count desync** — `downloaded=1` and `queue status=done` now updated in a single SQLite transaction; no more mismatch after restart
- **Cover images self-heal** — `serve_cover` and `serve_meta_cover` now clear stale local paths when files are missing; covers re-download on next discover visit. Migration `0024_fix_cover_paths` clears paths pointing at the old ephemeral location.

### Upgrading

```bash
docker compose pull
docker compose up -d
```

Migrations run automatically. Chapters previously downloaded to the wrong location will need to be re-downloaded — queue them again from the manga detail page.

---

## [0.0.6] — 2026-05-18

### Features

- **Trending section** — full trending shelf on the Discover page with source fan-out
- **Title metadata cache** — covers downloaded eagerly to local disk; CDN-gating transparent to client

### Bug fixes

- Mangapill thumbnails disappearing — `serve_meta_cover` now falls back to CDN proxy on missing local file
- Noisy sync and idle queue polling logs reduced

### Other

- Unit tests for core API handlers

---

## [0.0.5] — 2026-05-17

### Features

- **Toonily** source plugin (manhwa scraper, explicit content detection)
- **Comick** source plugin (manga / manhwa / manhua, multi-language)
- **Royal Road** source plugin (web fiction / novels)
- **NovelFull** source plugin (xianxia / wuxia novels)
- Light novel reader — Markdown text rendering
- Deploy documentation (`docs/deploy/docker-compose.md`, `docs/deploy/nginx.md`)
- Image proxy `Referer` header — fixes CDN hotlink protection on scraped sources

### Bug fixes

- Noisy sync log spam on idle sources

---

## [0.0.2] — 2026-05-17

### Features

- **External source plugin system** — sources are HTTP servers; no recompile needed to add a source
- **Mangapill** source plugin (first scraper source, manga)
- Multi-source fan-out search and chapter sync
- Per-user explicit content access control
- Content-type filtering (manga / manhwa / manhua / all)

---

## [0.0.1] — 2026-05-16

### Initial release

- Rust/Axum API server, React SPA, SQLite via SQLx
- MangaDex source (manga / manhwa / manhua / one-shot)
- Library — add series, auto-sync chapters, download for offline reading
- Web reader — paged and scroll modes
- Multi-user support with per-user reading progress
- Background chapter downloader and sync scheduler
- Docker image with nginx + Rust binary
