# Mangapill Source Plugin

A source plugin for [*ARRgh](../../README.md) that provides manga from [Mangapill](https://mangapill.com).

Implements the [Source Plugin Protocol](../../docs/adr/0004-external-source-plugin-protocol.md).

No FlareSolverr required — Mangapill uses standard HTTP (User-Agent + Referer).

---

## Content types

`manga`

---

## Running

### With Docker Compose (recommended)

Included in the root `docker-compose.yml` — starts automatically alongside arrgh. Mangapill auto-registers via `PLUGIN_URLS=http://mangapill:4000` on first boot.

### Locally (dev)

```bash
cd plugins/mangapill
npm install
npm run dev        # tsx watch mode
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
| `API_KEY` | — | Require `Authorization: Bearer <key>` on all requests |

Copy `.env.example` to `.env` and edit as needed.

---

## Endpoints

| Method | Path | Description |
|---|---|---|
| `GET` | `/info` | Plugin metadata (id, name, content_types) |
| `GET` | `/search?q=` | Search Mangapill by title |
| `GET` | `/trending` | Popular titles |
| `GET` | `/manga/:id/meta` | Description, cover, chapter count |
| `GET` | `/manga/:id/chapters` | Full chapter list |
| `GET` | `/chapter/:id/pages` | Page image URLs |
| `GET` | `/cover?url=` | Fetch CDN image with correct `Referer: https://mangapill.com` |

The `/cover` endpoint is used by arrgh when downloading covers for the Title Metadata Cache — Mangapill's CDN requires a `Referer` header that the generic proxy doesn't set.

---

## Notes

- Chapter `source_id` format: `{numeric_id}/{slug}` (e.g. `2-11182000/one-piece-chapter-1182`)
- Lazy-loaded images use `data-src` attributes — the plugin reads these correctly
- Mangapill has no authentication or rate limiting; scraping is straightforward
