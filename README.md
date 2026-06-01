# *ARRgh!
[![CI](https://github.com/t2vi/arrgh/actions/workflows/ci.yml/badge.svg)](https://github.com/t2vi/arrgh/actions/workflows/ci.yml) [![GHCR](https://github.com/t2vi/arrgh/actions/workflows/ghcr.yml/badge.svg)](https://github.com/t2vi/arrgh/actions/workflows/ghcr.yml) [![Docs-site](https://github.com/t2vi/arrgh/actions/workflows/docs-site.yml/badge.svg)](https://github.com/t2vi/arrgh/actions/workflows/docs-site.yml)
[![E2e](https://github.com/t2vi/arrgh/actions/workflows/e2e.yml/badge.svg)](https://github.com/t2vi/arrgh/actions/workflows/e2e.yml)

**v0.1.4-beta** · A self-hosted East Asian comics and novel manager, downloader, and reader for your home server. Supports manga, manhwa, manhua, and light novels from multiple sources via a plugin system. Built to run on a NAS, Raspberry Pi, or any always-on box.

> ⚠️ **Port change (v0.1.3+)** — the host-exposed port is now **8282** (was 8080 in v0.1.2 and earlier). Update any firewall rules, bookmarks, or reverse proxy configs. The internal container port remains 8080 — only the host-side mapping changed.

> I'm a solo dev who built this for myself — tired of juggling browser tabs, download scripts, and folder structures just to keep up with series. If you find it useful or want to contribute, you're very welcome. See [Contributing](#contributing).

---

## Features

- **Discover** powered by [MangaUpdates](https://www.mangaupdates.com/) — consistent metadata (titles, covers, descriptions, tags, authors) from a single authoritative source; [E-Hentai](https://e-hentai.org/) for explicit titles
- **Trending lanes** — Home screen shows 4 independent trending rows: Manga (MangaUpdates), Manhwa, Manhua, and Adult Manhwa (AniList); each lane caches independently
- Title aliases from MangaUpdates associated names — improves cross-source matching for series with multiple romanisations
- Chapters aggregated across all registered sources — completeness doesn't depend on any one source being up to date
- Automatic download fallback — if the preferred source fails, arrgh tries the next by priority
- Hentai source routing — explicit sources only matched for titles tagged `hentai`; non-explicit sources skipped for them
- Source plugin system — add new download sources without recompiling or redeploying
- Browse and install community plugins from the Settings UI
- Download chapters to your server for offline reading
- Real-time download progress with per-chapter percentage bars
- Live sync progress — library card and title detail page show step-by-step sync status while building
- Sync warnings — amber badge when a source couldn't be matched; re-sync to retry
- Web reader (paged or scroll mode for comics; prose mode for novels)
- Multi-user support — per-user libraries with shared file storage, per-user reading progress
- Auto-download new chapters on a schedule
- Explicit content controls — admin grants access per user; 18+ badge shown on all title cards (library, home, Discover, trending)
- Shared download queue — visible to all users, members cancel own items, admins cancel any

---

## Quick start (Docker)

```bash
curl -O https://raw.githubusercontent.com/t2vi/arrgh/main/docker-compose.yml
docker compose up -d
```

Open `http://<your-server-ip>:8282` — the setup wizard runs on first launch.

> **Upgrading from v0.1.2 or earlier?** The host port changed from `8080` to `8282`. Run `docker compose pull && docker compose up -d` and update any firewall rules or reverse proxy configs pointing to the old port.

The default Compose file includes the **Mangapill** and **MangaDex** plugins. They auto-register on first boot via `PLUGIN_URLS` — no manual configuration needed.

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

## Sources

*ARRgh! uses a plugin system for content sources. Each source is an HTTP server implementing the Source Plugin Protocol.

### Bundled plugins

All default sources compile into a single **plugin-host** container — no per-plugin ports or sidecars:

| Source | Content | Directory | Notes |
|---|---|---|---|
| **Mangapill** | Manga | `plugins/mangapill/` | |
| **MangaDex** | Manga, Manhwa, Manhua, One-shot | `plugins/mangadex/` | |
| **Toonily** | Manhwa | `plugins/toonily/` | CF-protected — uses CloakBrowser |
| **NovelFull** | Novel | `plugins/novelfull/` | CF-protected — uses CloakBrowser |
| **nhentai** | Hentai doujinshi | `plugins/nhentai/` | CF-protected — uses CloakBrowser; explicit-only source |

CF-protected plugins route through the **CloakBrowser** sidecar (stealth Chromium, source-level fingerprint patches). Plugin Host holds the CDP connection; plugins call `ctx.getBrowser()` via `PluginContext`.

### Adding a source

1. Write an HTTP server implementing the Source Plugin Protocol
2. Run it (locally or as a Docker service)
3. Register it: **Settings → Sources → Add** (or set `PLUGIN_URLS` for auto-registration on startup)

### Source Plugin Protocol

Plugins are **download-only backends**. Metadata (search, descriptions, covers, trending) comes from MangaUpdates for standard titles, or E-Hentai for explicit titles — plugins only need to serve chapter lists and page content.

Every plugin must implement:

```
GET /info                         → { id, name, default_explicit, content_types }
GET /manga/:source_id/chapters    → [ChapterResult]
GET /chapter/:source_id/pages     → [image_url]
```

Optional:

```
GET /chapter/:source_id/text      → Markdown string (novel/light-novel chapters only)
```

Plugins can be written in any language. See `plugins/mangadex/` (API-backed) and `plugins/toonily/` (scraper + CloakBrowser) for reference implementations.

> **Note**: older plugins that implement `/search`, `/trending`, `/meta`, or `/cover` continue to work — arrgh ignores those routes but doesn't reject plugins that expose them.

---

## Architecture

```
arrgh/
├── server/          # .NET 9 / ASP.NET Core API server
├── web/             # React + TypeScript SPA
├── plugin-host/     # Node.js plugin host (loads compiled plugin bundles)
└── plugins/         # Plugin source bundles (esbuild → single .js)
    ├── mangadex/
    ├── mangapill/
    ├── toonily/
    ├── novelfull/
    ├── nhentai/
    └── manga18fx/
```

- **Backend**: .NET 10, ASP.NET Core, EF Core (SQLite)
- **Frontend**: React 18, TypeScript, Vite, Tailwind
- **Plugins**: Node.js bundles loaded by plugin-host; CF-protected sources use CloakBrowser via CDP

---

## Contributing

Issues and PRs are welcome. A few things to know:

- This is a personal project — I may be slow to review, but I do look at everything
- Check open issues before starting large features; comment to claim one
- Run `dotnet test` (server) and `npm test` (web) before submitting
- Follow the existing code style — see `CLAUDE.md` for dev setup

No CLA, no process overhead. Just open a PR.

---

## Roadmap

See [ROADMAP.md](ROADMAP.md).

---

## License

[GNU GPL v3](LICENSE)
