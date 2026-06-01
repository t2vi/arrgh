# MangaDex Source Plugin

A source plugin for [*ARRgh](../../README.md) that provides manga, manhwa, and manhua from [MangaDex](https://mangadex.org).

Implements the [Source Plugin Protocol](../../docs/adr/0004-external-source-plugin-protocol.md).

---

## Content types

`manga`, `manhwa`, `manhua`, `one-shot`

---

## Running

### With Docker Compose (recommended)

Bundled into the **plugin-host** container — starts automatically with `docker compose up -d`. No separate container or manual registration needed.

### Locally (dev)

Build the bundle and hot-reload it into a running plugin-host:

```bash
cd plugins/mangadex
npm install
npm run build   # esbuild → bundles/mangadex.js
docker cp bundles/mangadex.js gwarr-plugin-host-1:/app/bundles/mangadex.js
```

Plugin-host hot-reloads when the file changes — no container restart needed.

---

## Configuration

| Variable | Default | Description |
|---|---|---|
| `PORT` | `4001` | Listen port |
| `LANGUAGES` | `en` | Comma-separated translated languages (e.g. `en,fr,pt-br`) |
| `API_KEY` | — | Require `Authorization: Bearer <key>` on all requests |

Copy `.env.example` to `.env` and edit as needed.

---

## Endpoints

| Method | Path | Description |
|---|---|---|
| `GET` | `/info` | Plugin metadata (id, name, content_types) |
| `GET` | `/search?q=` | Search MangaDex by title |
| `GET` | `/trending` | Popular titles (by follow count) |
| `GET` | `/manga/:id/meta` | Description, cover, chapter count |
| `GET` | `/manga/:id/chapters` | All chapters (paginated internally, language-filtered) |
| `GET` | `/chapter/:id/pages` | Page image URLs from MangaDex at-home CDN |

---

## Notes

- Chapters are deduplicated by number — one entry per chapter, first scanlation group wins
- Pages are fetched from the MangaDex at-home server (high-res `/data/` path, not data-saver)
- Chapter pagination is handled internally (500 chapters per request to MangaDex API)
- MangaDex rate limits apply — avoid hammering the API; the arrgh scheduler's default 6-hour sync interval is safe

### Known limitation

MangaDex's at-home server terms ask clients to report page view success/failure to `${baseUrl}/report`. This plugin does not implement reporting. Pages load correctly but technically violates at-home server terms.
