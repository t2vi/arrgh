# *ARRgh!
[![CI](https://github.com/t2vi/arrgh/actions/workflows/ci.yml/badge.svg)](https://github.com/t2vi/arrgh/actions/workflows/ci.yml) [![Docker](https://github.com/t2vi/arrgh/actions/workflows/docker.yml/badge.svg)](https://github.com/t2vi/arrgh/actions/workflows/docker.yml)

A self-hosted manga manager, downloader, and reader for your home server. Supports manga, manhwa, and manhua from multiple sources via a plugin system. Built to run on a NAS, Raspberry Pi, or any always-on box.

> I'm a solo dev who built this for myself — tired of juggling browser tabs, download scripts, and folder structures just to keep up with series. If you find it useful or want to contribute, you're very welcome. See [Contributing](#contributing).

---

## Features

- Browse and search manga from multiple sources simultaneously
- Source plugin system — add new sources without recompiling or redeploying
- Download chapters to your server for offline reading
- Web reader (paged or scroll mode) and Flutter app (Android / Firestick / tablet)
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

The default Compose file includes the **MangaDex plugin** (manga + manhwa + manhua). It auto-registers on first boot via `PLUGIN_URLS` — no manual configuration needed.

Or run just the server (Mangapill built-in only):

```bash
docker run -d \
  --name arrgh \
  -p 8080:8080 \
  -v arrgh_data:/data \
  ghcr.io/t2vi/arrgh:latest
```

See [docs/deploy/docker-compose.md](docs/deploy/docker-compose.md) for full configuration.

---

## Sources

*ARRgh uses a plugin system for content sources. Each source is an HTTP server implementing the [Source Plugin Protocol](docs/adr/0004-external-source-plugin-protocol.md).

### Built-in (compiled-in)

| Source | Content | Notes |
|---|---|---|
| **Mangapill** | Manga | Default, no setup required |

### Plugins (external HTTP servers)

| Source | Content | Directory |
|---|---|---|
| **MangaDex** | Manga, Manhwa, Manhua | `plugins/mangadex/` |

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
GET /manga/:source_id/meta
```

Plugins can be written in any language. See `plugins/mangadex/` for a TypeScript/Express reference implementation.

---

## Architecture

```
arrgh/
├── server/          # Rust / Axum API server
├── web/             # React + TypeScript SPA
├── app/             # Flutter app (Android / Firestick / tablet)
└── plugins/
    └── mangadex/    # MangaDex source plugin (TypeScript / Express)
```

- **Backend**: Rust, Axum, SQLx (SQLite), Tokio
- **Frontend**: React 18, TypeScript, Vite, TanStack Query, Tailwind
- **Mobile**: Flutter, Riverpod
- **Plugins**: any language, HTTP server, Source Plugin Protocol

---

## Contributing

Issues and PRs are welcome. A few things to know:

- This is a personal project — I may be slow to review, but I do look at everything
- Check open issues before starting large features; comment to claim one
- Run `cargo test` (server) and `flutter test` (app) before submitting
- Follow the existing code style — see [server/README.md](server/README.md) and [web/README.md](web/README.md) for dev setup

No CLA, no process overhead. Just open a PR.

---

## Roadmap

**Sources**
- [x] Source plugin system — add sources without recompiling
- [x] MangaDex (manga / manhwa / manhua)
- [ ] Manhwa-specific vertical/webtoon reader layout
- [ ] Scanlation group preference per manga

**Reader**
- [ ] Light novel reader (text-based, EPUB/TXT support)
- [ ] Keyboard and remote shortcuts in web reader
- [ ] Reading statistics (time spent, chapters per week)

**Library**
- [ ] Metadata editing (title, cover, tags)
- [ ] CBZ / CBR local import
- [ ] Backup and export (library + reading progress)

**Integrations**
- [ ] Push notifications for new chapters
- [ ] Webhook on new chapter download

**Infrastructure**
- [ ] PostgreSQL support alongside SQLite

These are the items I think are most important — open one to propose or claim.

---

## License

[GNU GPL v3](LICENSE)
