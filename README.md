# *ARRgh!
[![CI](https://github.com/t2vi/arrgh/actions/workflows/ci.yml/badge.svg)](https://github.com/t2vi/arrgh/actions/workflows/ci.yml) [![GHCR](https://github.com/t2vi/arrgh/actions/workflows/ghcr.yml/badge.svg)](https://github.com/t2vi/arrgh/actions/workflows/ghcr.yml) [![Docs-site](https://github.com/t2vi/arrgh/actions/workflows/docs-site.yml/badge.svg)](https://github.com/t2vi/arrgh/actions/workflows/docs-site.yml)
[![E2e](https://github.com/t2vi/arrgh/actions/workflows/e2e.yml/badge.svg)](https://github.com/t2vi/arrgh/actions/workflows/e2e.yml)

**v1.3.0** · A self-hosted comics and web-novel manager, downloader, and reader for your home server. Supports manga, manhwa, manhua, novels (translated and English-original), and hentai from multiple sources via a plugin system. Built to run on a NAS, Raspberry Pi, or any always-on box.

> I'm a solo dev who built this for myself — tired of juggling browser tabs, download scripts, and folder structures just to keep up with series. If you find it useful or want to contribute, you're very welcome. See [Contributing](#contributing).

---

## Features

- **Discover** — fan-out search across 7 authorities: MangaUpdates (manga), AniList (manhwa), MangaDex (manhua), NovelUpdates + WuxiaWorld (translated novels), Royal Road (English-original novels), nhentai (hentai). Results deduplicated by authority precedence; live per-source progress, with results streaming in as each source answers
- **Trending lanes** — Home screen shows 4 independent trending rows: Manga (MangaUpdates), Manhwa, Manhua, and Adult Manhwa (AniList); each lane caches independently
- Title aliases from MangaUpdates associated names — improves cross-source matching for series with multiple romanisations
- Chapters aggregated across all registered sources — completeness doesn't depend on any one source being up to date
- Automatic download fallback — if the preferred source fails, arrgh tries the next by priority
- Scheduled re-sync + auto-download — every **Sync interval** (Settings, 1–24 h) library titles re-sync; newly found chapters are queued when auto-download is on (globally, or per title: Global / Always / Never)
- Parallel downloads — **Download workers** (Settings → Downloads, 1–10) chapters at once; changes apply without a restart
- A stuck source can't stall search, sync or downloads — every plugin call is time-limited
- Hentai source routing — explicit sources only matched for titles tagged `hentai`; non-explicit sources skipped for them
- Source plugin system — add new download sources without recompiling or redeploying
- Browse and install community plugins from the Settings UI
- Update a broken source from Settings without upgrading the app: the plugin catalog is read live, each download is checked against its sha256 before it loads, and **Revert** goes back to the version bundled in the image
- Download chapters to your server for offline reading
- Real-time download progress with per-chapter percentage bars
- **Library sort & filter** — sort by recently added, title A–Z/Z–A, or year; filter by content type (manga/manhwa/manhua/novel) and status (ongoing/completed/hiatus/cancelled); active filter count badge
- Live sync progress — library card and title detail page show step-by-step sync status while building
- Sync warnings — amber badge when a source couldn't be matched; re-sync to retry
- Web reader (paged or scroll mode for comics; prose mode for novels)
- Multi-user support — per-user libraries with shared file storage, per-user reading progress
- Explicit content controls — admin grants access per user; 18+ badge shown on all title cards (library, home, Discover, trending)
- Shared download queue — visible to all users, members cancel own items, admins cancel any

---

## Quick start (Docker)

```bash
curl -O https://raw.githubusercontent.com/t2vi/arrgh/main/docker-compose.yml
docker compose up -d
```

Open `http://<your-server-ip>:8282` — the setup wizard runs on first launch.

The default Compose file runs **plugin-host** (all bundled sources, below) and the **CloakBrowser** sidecar for Cloudflare-protected sites. The bundled sources register on first boot — no manual configuration needed.

See [docs/deploy/docker-compose.md](docs/deploy/docker-compose.md) for full configuration.

---

## Upgrading

```bash
docker compose pull
docker compose up -d
```

Migrations run automatically on startup. No manual DB steps needed.

**From v0.1.2 or earlier** — the host port changed from `8080` to `8282`. Update firewall rules, bookmarks, and any reverse proxy config that referenced `:8080`.

---

## Portainer

Deploy the same Compose stack through Portainer's UI instead of the CLI: **Stacks → Add stack → Repository**, pointing at this repo's `docker-compose.yml`, or paste the file into the Web editor. Same image, same env vars — set them through Portainer's Environment variables editor.

See [docs/deploy/portainer.md](docs/deploy/portainer.md) for the full walkthrough.

---

## Kubernetes

Best-effort, community-supported — not exercised by this project's CI. Example manifests (`k8s/`) run a single replica with a persistent volume for the database and downloads directory (SQLite is single-writer, so this isn't horizontally scalable).

```bash
kubectl apply -f k8s/
```

See [docs/deploy/kubernetes.md](docs/deploy/kubernetes.md) for storage/replica rationale and ingress notes.

---

## Sources

*ARRgh! uses a plugin system for content sources. Each source is an HTTP server implementing the Source Plugin Protocol.

### Bundled plugins

All default sources compile into a single **plugin-host** container — no per-plugin ports or sidecars:

| Source | Content | Directory | Notes |
|---|---|---|---|
| **Mangapill** | Manga | `plugins/mangapill/` | |
| **MangaDex** | Manga, Manhwa, Manhua, One-shot | `plugins/mangadex/` | |
| **Toonily** | Manhwa | `plugins/toonily/` | CF-protected — uses CloakBrowser |
| **AsuraScans** | Manhwa | `plugins/asurascans/` | |
| **Manga18fx** | Manhwa (explicit) | `plugins/manga18fx/` | `default_explicit=true` |
| **NovelFull** | Novel | `plugins/novelfull/` | CF-protected — uses CloakBrowser |
| **NovelFull.net** | Novel | `plugins/novelfullnet/` | novelfull.net — same site, different catalog (e.g. The Primal Hunter); CF-protected — uses CloakBrowser |
| **WuxiaWorld** | Novel | `plugins/wuxiaworld/` | Official API — no CF protection |
| **Royal Road** | Novel (English originals) | `plugins/royalroad/` | Direct fetch — no CF protection; also a Discover authority |
| **nhentai** | Hentai doujinshi | `plugins/nhentai/` | Direct API, CloakBrowser fallback when challenged; explicit-only source |

`plugins/novelupdates/` is not a download source — it backs the NovelUpdates Discover authority.

CF-protected plugins route through the **CloakBrowser** sidecar (stealth Chromium, source-level fingerprint patches). Plugin Host holds the CDP connection; plugins call `ctx.getBrowser()` via `PluginContext`.

### Adding a source

The quickest way is a bundle plugin. Create a repo from
**[arrgh-plugin-template](https://github.com/t2vi/arrgh-plugin-template)**: it has the
[plugin SDK](https://github.com/t2vi/arrgh-plugin-sdk) (types, `arrgh-plugin build`, contract tests),
CI, and a release workflow that publishes `<id>.js`, its sha256, and the entry to add to
`plugin-index/index.json`.

Or run your own HTTP server:

1. Write an HTTP server implementing the Source Plugin Protocol
2. Run it (locally or as a Docker service)
3. Register it: **Settings → Sources → Add** (or set `PLUGIN_URLS` for auto-registration on startup)

### Source Plugin Protocol

Plugins are **download-only backends**. Metadata (search, descriptions, covers, trending) comes from the discover fan-out authorities (MangaUpdates, AniList, MangaDex, NovelUpdates, WuxiaWorld, Royal Road, nhentai) — plugins only need to serve chapter lists and page content.

Every plugin must implement:

```
GET /info                         → { id, name, version, default_explicit, content_types }
GET /manga/:source_id/chapters    → [ChapterResult]
GET /chapter/:source_id/pages     → [image_url]
```

Optional:

```
GET /chapter/:source_id/text      → Markdown string (novel/light-novel chapters only)
```

Plugins can be written in any language. See `plugins/mangadex/` (API-backed) and `plugins/toonily/` (scraper + CloakBrowser) for reference implementations.

`version` is the bundle's own version; Settings shows it as the loaded version (missing = "unknown").

plugin-host bounds every plugin call (`PLUGIN_CALL_TIMEOUT_MS`, default 180 s) and answers `504` when one runs out of time.

> **Note**: older plugins that implement `/search`, `/trending`, `/meta`, or `/cover` continue to work — arrgh ignores those routes but doesn't reject plugins that expose them.

---

## Architecture

```
arrgh/
├── server/          # Rust / axum API server
├── web-svelte/      # Svelte 5 + TypeScript SPA
├── plugin-host/     # Node.js plugin host (loads compiled plugin bundles)
├── plugin-index/    # index.json — plugin catalog shipped in the image
├── scripts/         # dev-up.sh (one-command dev stack), sync-plugins.sh
└── plugins/         # Plugin source bundles (esbuild → single .js)
    ├── mangadex/  mangapill/  toonily/  asurascans/  manga18fx/
    ├── novelfull/  novelfullnet/  wuxiaworld/  royalroad/  novelupdates/
    ├── nhentai/
    └── fixture/     # e2e test plugin — never shipped
```

> Plugins are moving to one repo each, with one-click updates from Settings ([#199](https://github.com/t2vi/arrgh/issues/199)).

- **Backend**: Rust, axum, sqlx (SQLite)
- **Frontend**: Svelte 5 (runes), TypeScript, Vite, Tailwind
- **Plugins**: Node.js bundles loaded by plugin-host; CF-protected sources use CloakBrowser via CDP

---

## Contributing

Issues and PRs are welcome. A few things to know:

- This is a personal project — I may be slow to review, but I do look at everything
- Check open issues before starting large features; comment to claim one
- Run `cargo test` (server), `npm test` in `web-svelte/` and `plugin-host/` before submitting
- Follow the existing code style — see `CLAUDE.md` for dev setup

### Local development (quick start)

```bash
./scripts/dev-up.sh
```

Starts the Rust API server, Vite web server, plugin-host, and CloakBrowser (for CF-protected
sources) together, `Ctrl-C` stops all four. Requires Rust, Node.js, and Docker or podman already
installed, with `plugin-host`/`web-svelte` dependencies installed once (`npm install` in each).
For running/restarting one service individually instead, see `CLAUDE.md`'s Dev Workflow section.

No CLA, no process overhead. Just open a PR.

---

## Roadmap

See [ROADMAP.md](ROADMAP.md).

---

## License

[GNU GPL v3](LICENSE)
