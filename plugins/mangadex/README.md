# MangaDex Source Plugin

A source plugin for [*ARRgh](../../README.md) that provides manga, manhwa, and manhua from [MangaDex](https://mangadex.org).

Implements the [Source Plugin Protocol](../../docs/adr/0004-external-source-plugin-protocol.md).

---

## Content types

`manga`, `manhwa`, `manhua`, `one-shot`

---

## Running

### With Docker Compose (recommended)

Included in the root `docker-compose.yml` — starts automatically alongside arrgh. MangaDex auto-registers via `PLUGIN_URLS=http://mangadex:4000` on first boot.

### Locally (dev)

```bash
cd plugins/mangadex
npm install
npm run dev        # ts-node watch mode
# or
npm run build && npm start
```

Plugin listens on `http://localhost:4000` by default.

**Important:** running the plugin is not enough — you must also register it with arrgh. Pick one:

**Option A — auto-register at startup** (recommended for dev)

Add to `server/.env`:

```
PLUGIN_URLS=http://localhost:4000
```

Then (re)start the arrgh server. The URL is inserted once; subsequent restarts skip it.

**Option B — register at runtime**

With both arrgh and the plugin running, open **Settings → Sources → Add** and enter `http://localhost:4000`. No restart needed.

---

## Configuration

| Variable | Default | Description |
|---|---|---|
| `PORT` | `4000` | Listen port |
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
