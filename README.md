# *ARRgh!
[![CI](https://github.com/t2vi/arrgh/actions/workflows/ci.yml/badge.svg)](https://github.com/t2vi/arrgh/actions/workflows/ci.yml) [![Docker](https://github.com/t2vi/arrgh/actions/workflows/docker.yml/badge.svg)](https://github.com/t2vi/arrgh/actions/workflows/docker.yml)

A self-hosted manga manager, downloader, and reader for your home server. Supports manga, manhwa, and manhua from multiple sources via a plugin system. Built to run on a NAS, Raspberry Pi, or any always-on box.

> I'm a solo dev who built this for myself — tired of juggling browser tabs, download scripts, and folder structures just to keep up with series. If you find it useful or want to contribute, you're very welcome. See [Contributing](#contributing).

---

## Features

- Browse and search manga from multiple sources simultaneously
- Source plugin system — add new sources without recompiling or redeploying
- Download chapters to your server for offline reading
- Web reader (paged or scroll mode)
- Multi-user support — per-user libraries with shared file storage, per-user reading progress
- Auto-download new chapters on a schedule
- Explicit content controls — admin grants access per user
- Content-type filtering — search only manga, manhwa, manhua, or all at once

---

## Quick start (Docker)

```bash
curl -O https://raw.githubusercontent.com/t2vi/arrgh/main/docker-compose.yml
docker compose up -d
```

Open `http://<your-server-ip>:8080` — the setup wizard runs on first launch.

The default Compose file includes the **Mangapill** and **MangaDex** plugins. They auto-register on first boot via `PLUGIN_URLS` — no manual configuration needed.

See [docs/deploy/docker-compose.md](docs/deploy/docker-compose.md) for full configuration.

---

## Sources

*ARRgh uses a plugin system for content sources. Each source is an HTTP server implementing the [Source Plugin Protocol](docs/adr/0004-external-source-plugin-protocol.md).

### Bundled plugins

All default sources run as external HTTP plugins — no compiled-in sources:

| Source | Content | Port | Directory | Notes |
|---|---|---|---|---|
| **Mangapill** | Manga | 4000 | `plugins/mangapill/` | |
| **MangaDex** | Manga, Manhwa, Manhua, One-shot | 4001 | `plugins/mangadex/` | |
| **Toonily** | Manhwa | 4002 | `plugins/toonily/` | Requires [FlareSolverr](https://github.com/FlareSolverr/FlareSolverr) sidecar |
| **Comick** | Manga, Manhwa, Manhua | 4003 | `plugins/comick/` | Requires [FlareSolverr](https://github.com/FlareSolverr/FlareSolverr) sidecar |
| **Royal Road** | Novel | 4004 | `plugins/royalroad/` | English web fiction |
| **NovelFull** | Novel | 4005 | `plugins/novelfull/` | Requires [FlareSolverr](https://github.com/FlareSolverr/FlareSolverr) sidecar |

### Adding a source

1. Write an HTTP server implementing the [Source Plugin Protocol](#source-plugin-protocol)
2. Run it (locally or as a Docker service)
3. Register it: **Settings → Sources → Add** (or set `PLUGIN_URLS` for auto-registration on startup)

### Source Plugin Protocol

Every plugin must implement:

```
GET /info                         → { id, name, default_explicit, content_types }
GET /search?q=<query>             → [MangaResult]
GET /manga/:source_id/chapters    → [ChapterResult]
GET /chapter/:source_id/pages     → [image_url]
```

Optional endpoints (gracefully skipped if absent):

```
GET /trending
GET /manga/:source_id/meta        → { description, cover_url, chapter_count, tags? }
GET /cover?url=<encoded_cdn_url>  → raw image bytes
GET /chapter/:source_id/text      → Markdown string (novel chapters only)
```

`/cover` lets a plugin fetch CDN images with source-specific headers (e.g. custom `Referer`). When absent, arrgh fetches directly with a browser User-Agent.

`tags` in meta is a comma-separated genre string. Include `"adult"` to signal explicit content — arrgh sets `is_explicit = true` on sync and hides the title from users without Explicit Permission.

Plugins can be written in any language. See `plugins/mangadex/` (API-backed) and `plugins/toonily/` (scraper + FlareSolverr) for reference implementations.

---

## Architecture

```
arrgh/
├── server/          # Rust / Axum API server
├── web/             # React + TypeScript SPA
├── plugin-host/     # Node.js plugin host (loads compiled plugin bundles)
└── plugins/         # Plugin source bundles (esbuild → single .js)
    ├── mangadex/
    ├── mangapill/
    ├── toonily/
    ├── comick/
    ├── royalroad/
    └── novelfull/
```

- **Backend**: Rust, Axum, SQLx (SQLite), Tokio
- **Frontend**: React 18, TypeScript, Vite, Tailwind
- **Plugins**: Node.js bundles loaded by plugin-host; CF-protected sources use CloakBrowser via CDP

---

## Contributing

Issues and PRs are welcome. A few things to know:

- This is a personal project — I may be slow to review, but I do look at everything
- Check open issues before starting large features; comment to claim one
- Run `cargo test` (server) before submitting
- Follow the existing code style — see [server/README.md](server/README.md) and [web/README.md](web/README.md) for dev setup

No CLA, no process overhead. Just open a PR.

---

## Roadmap

**Sources** <br />
✅ Source plugin system — add sources without recompiling <br />
✅ MangaDex (manga / manhwa / manhua / one-shot) <br />
✅ Mangapill (manga) <br />
✅ Toonily (manhwa) <br />
✅ Comick (manga / manhwa / manhua) <br />
✅ Royal Road (web fiction / novels) <br />
✅ NovelFull (xianxia / wuxia novels) <br />
✅ Multi-source fan-out — parallel search + trending, merged by title <br />
✅ Title metadata cache — covers eagerly downloaded locally, CDN-gating transparent to client <br />
✅ Library cover fix — internal meta-cover paths resolved to CDN URL at add-to-library time; `serve_cover` self-heals stale entries <br />

**Reader**<br />
✅ Light novel reader (text-based, Markdown) <br />
🔳 Novel reader typography controls (font size, line width, serif/sans) <br />
🔳 Keyboard and remote shortcuts in web reader <br />
🔳 Reading statistics (time spent, chapters per week) <br />

**Library** <br />
🔳 Metadata editing (title, cover, tags) <br />
🔳 CBZ / CBR local import <br />
🔳 Backup and export (library + reading progress) <br />

**Integrations** <br />
🔳 Push notifications for new chapters <br />
🔳 Webhook on new chapter download <br />

**Infrastructure** <br />
🔳 PostgreSQL support alongside SQLite <br />

These are the items I think are most important — open one to propose or claim.

---

## License

[GNU GPL v3](LICENSE)
