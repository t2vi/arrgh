# *ARRgh!
[![CI](https://github.com/t2vi/arrgh/actions/workflows/ci.yml/badge.svg)](https://github.com/t2vi/arrgh/actions/workflows/ci.yml) [![GHCR](https://github.com/t2vi/arrgh/actions/workflows/ghcr.yml/badge.svg)](https://github.com/t2vi/arrgh/actions/workflows/ghcr.yml) [![Docs-site](https://github.com/t2vi/arrgh/actions/workflows/docs-site.yml/badge.svg)](https://github.com/t2vi/arrgh/actions/workflows/docs-site.yml)
[![E2e](https://github.com/t2vi/arrgh/actions/workflows/e2e.yml/badge.svg)](https://github.com/t2vi/arrgh/actions/workflows/e2e.yml)

**v1.4.0** · A self-hosted comics and web-novel manager, downloader, and reader for your home server. Supports manga, manhwa, manhua, novels (translated and English-original), and hentai from multiple sources via a plugin system. Built to run on a NAS, Raspberry Pi, or any always-on box.

> I'm a solo dev who built this for myself — tired of juggling browser tabs, download scripts, and folder structures just to keep up with series. If you find it useful or want to contribute, you're very welcome. See [Contributing](#contributing).

---

## Quick start

### Docker (recommended)

```bash
curl -O https://raw.githubusercontent.com/t2vi/arrgh/main/docker-compose.yml
docker compose up -d
```

Open `http://<your-server-ip>:8282` — the setup wizard runs on first launch. The default Compose
file runs **plugin-host** (all bundled sources) and the **CloakBrowser** sidecar for
Cloudflare-protected sites — nothing else to configure. See
[docs/deploy/docker-compose.md](docs/deploy/docker-compose.md) for full configuration, or
[Portainer](#portainer) / [Kubernetes](#kubernetes) below for those deployment paths.

To upgrade later, from the same directory:

```bash
docker compose pull
docker compose up -d
```

Migrations run automatically on startup — no manual DB steps needed.

> **From v0.1.2 or earlier** — the host port changed from `8080` to `8282`. Update firewall rules, bookmarks, and any reverse proxy config that referenced `:8080`.

### Run from source (local dev)

```bash
./scripts/dev-up.sh
```

Starts the Rust API server, Vite web server, plugin-host, and CloakBrowser together —
`Ctrl-C` stops all four. Requires Rust, Node.js, and Docker or podman already installed, with
`plugin-host`/`web-svelte` dependencies installed once (`npm install` in each). See
[Contributing](#contributing) for the per-service steps and test commands.

---

## Features

- **Search once, find it everywhere** — one search box fans out across 7 sites (MangaUpdates,
  AniList, MangaDex, NovelUpdates, WuxiaWorld, Royal Road, nhentai) and dedupes the results, so
  you're not scrolling past the same series ten times under ten different covers
- **Your whole reading list in one place** — manga, manhwa, manhua, translated novels,
  English-original web novels, and hentai, tracked side by side instead of six different bookmark folders
- **Never miss a chapter** — arrgh checks your sources on a schedule and can download new
  chapters the moment they're out, so your library is caught up before you open the app
- **A dead source doesn't kill your backlog** — chapters are pooled across every source that
  carries a title, so if one site goes down, blocks scrapers, or gets abandoned, downloads
  automatically fall back to the next
- **Fix a broken source yourself, no app update required** — scrapers break when sites change
  their layout; update or roll back a source from Settings in a couple clicks instead of waiting
  on a release
- **Read the way that fits the content** — paged or scroll mode for comics, a distraction-free
  prose reader for novels
- **Built for a household, not just you** — everyone gets their own library and reading progress
  on shared storage, with admin-controlled access to explicit content and a shared download
  queue everyone can see
- **Take it with you** — download chapters to your server for offline reading, with real-time
  per-chapter progress while it happens
- **Open to new sources** — anyone can write a new source plugin without recompiling or
  redeploying arrgh itself (see [Sources](#sources))

<details>
<summary>Also under the hood</summary>

Title aliasing for better cross-source matching · configurable sync interval and parallel
download workers · per-plugin call timeouts so one stuck source can't stall search/sync/downloads ·
library sort/filter with active-filter badge · live step-by-step sync status on library/detail
cards · sync-warning badges for unmatched sources.

</details>

---

## Pointing downloads at a NAS

Downloads and the database live under the `arrgh` container's `/data` volume
(`docker-compose.yml`'s `arrgh_data`). To store them on a NAS instead of local disk, mount the NAS
share on the Docker host first (NFS or SMB, whichever your NAS exposes), then bind-mount that path
in place of the named volume:

```yaml
services:
  arrgh:
    volumes:
      - /mnt/nas/arrgh:/data   # /mnt/nas/arrgh is the NAS share mounted on the host
```

`/mnt/nas/arrgh` must already be mounted and writable by the container's user before `docker
compose up` — arrgh doesn't mount network shares itself, only the local path it's given. See
[docs/deploy/docker-compose.md](docs/deploy/docker-compose.md#production-checklist) for the full
volume layout (DB + downloads paths) and backup notes.

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

| Source | Content | Repo | Notes |
|---|---|---|---|
| **Mangapill** | Manga | [`arrgh-plugin-mangapill`](https://github.com/t2vi/arrgh-plugin-mangapill) | |
| **MangaDex** | Manga, Manhwa, Manhua, One-shot | [`arrgh-plugin-mangadex`](https://github.com/t2vi/arrgh-plugin-mangadex) | |
| **Toonily** | Manhwa | [`arrgh-plugin-toonily`](https://github.com/t2vi/arrgh-plugin-toonily) | CF-protected — uses CloakBrowser |
| **AsuraScans** | Manhwa | [`arrgh-plugin-asurascans`](https://github.com/t2vi/arrgh-plugin-asurascans) | |
| **Manga18fx** | Manhwa (explicit) | [`arrgh-plugin-manga18fx`](https://github.com/t2vi/arrgh-plugin-manga18fx) | `default_explicit=true` |
| **NovelFull** | Novel | [`arrgh-plugin-novelfull`](https://github.com/t2vi/arrgh-plugin-novelfull) | CF-protected — uses CloakBrowser |
| **NovelFull.net** | Novel | [`arrgh-plugin-novelfullnet`](https://github.com/t2vi/arrgh-plugin-novelfullnet) | novelfull.net — same site, different catalog (e.g. The Primal Hunter); CF-protected — uses CloakBrowser |
| **WuxiaWorld** | Novel | [`arrgh-plugin-wuxiaworld`](https://github.com/t2vi/arrgh-plugin-wuxiaworld) | Official API — no CF protection |
| **Royal Road** | Novel (English originals) | [`arrgh-plugin-royalroad`](https://github.com/t2vi/arrgh-plugin-royalroad) | Direct fetch — no CF protection; also a Discover authority |
| **nhentai** | Hentai doujinshi | [`arrgh-plugin-nhentai`](https://github.com/t2vi/arrgh-plugin-nhentai) | Direct API, CloakBrowser fallback when challenged; explicit-only source |
| **NovelUpdates** | — (metadata authority only) | [`arrgh-plugin-novelupdates`](https://github.com/t2vi/arrgh-plugin-novelupdates) | Not a download source — backs the NovelUpdates Discover authority (`info.metadata_only=true`); CF-protected |

Every plugin lives in its own `t2vi/arrgh-plugin-<id>` repo (spec 031, ADR 0035) — this repo
no longer contains plugin source at all (only the e2e `plugins/fixture/`). The image fetches each
bundled plugin from its own repo's published release at build time, checksum- and
version-verified (`scripts/fetch-plugin-bundles.mjs`).

CF-protected plugins route through the **CloakBrowser** sidecar (stealth Chromium, source-level fingerprint patches). Plugin Host holds the CDP connection; plugins call `ctx.getBrowser()` via `PluginContext`.

### Adding a source

The quickest way is a bundle plugin. Create a repo from
**[arrgh-plugin-template](https://github.com/t2vi/arrgh-plugin-template)**: it has the
**[arrgh-plugin-sdk](https://github.com/t2vi/arrgh-plugin-sdk)** (types, `arrgh-plugin build`,
contract tests), CI, and a release workflow that publishes `<id>.js`, its sha256, and the entry to
add to `plugin-index/index.json`.

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

Plugins can be written in any language. See [`arrgh-plugin-mangadex`](https://github.com/t2vi/arrgh-plugin-mangadex) (API-backed) and [`arrgh-plugin-toonily`](https://github.com/t2vi/arrgh-plugin-toonily) (scraper + CloakBrowser) for reference implementations, or start from [`arrgh-plugin-template`](https://github.com/t2vi/arrgh-plugin-template).

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
├── scripts/         # dev-up.sh (one-command dev stack), sync-plugins.sh, fetch-plugin-bundles.mjs
└── plugins/         # Only the e2e fixture/ plugin — every real plugin lives in its own
    └── fixture/     # arrgh-plugin-<id> repo now, fetched from its release at build time
```

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

To run/restart one service individually instead of the full `./scripts/dev-up.sh` stack (see
[Quick start](#quick-start) above):

```bash
# Terminal 1 — API server
cd server && cargo run

# Terminal 2 — Web dev server
cd web-svelte && npm run dev

# Terminal 3 — Plugin host (needed for source browsing + chapter pages)
cd plugin-host && npm install && ../scripts/sync-plugins.sh && npm start
```

Web runs at `http://localhost:5173`, API at `http://localhost:3001`, Plugin Host at
`http://localhost:4000`. See `CLAUDE.md`'s Dev Workflow section for CF-protected source testing
(CloakBrowser) and other environment details.

No CLA, no process overhead. Just open a PR.

---

## Roadmap

See [ROADMAP.md](ROADMAP.md).

---

## License

[GNU GPL v3](LICENSE)
