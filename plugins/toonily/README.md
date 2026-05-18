# Toonily Source Plugin

A source plugin for [*ARRgh](../../README.md) that provides manhwa from [Toonily](https://toonily.com).

Implements the [Source Plugin Protocol](../../docs/adr/0004-external-source-plugin-protocol.md).

Toonily is behind Cloudflare Bot Management — this plugin requires a [FlareSolverr](https://github.com/FlareSolverr/FlareSolverr) sidecar to bypass the JS challenge.

---

## Content types

`manhwa`

---

## Explicit content

Toonily mixes SFW and adult manhwa. This plugin returns `"Adult"` in the `tags` field for titles that carry Toonily's Adult genre. arrgh detects this tag on add and sets `is_explicit = true` automatically — those titles are hidden from users without Explicit Permission.

`default_explicit` is `false` — SFW titles (Solo Leveling, Tower of God, etc.) are visible to all users.

---

## Running

### With Docker Compose (recommended)

Included in the root `docker-compose.yml`. FlareSolverr and Toonily start automatically alongside arrgh. Toonily auto-registers via `PLUGIN_URLS` on first boot.

```bash
docker compose up
```

### Locally (dev)

**Step 1 — start FlareSolverr:**

```bash
docker run -d --name flaresolverr -p 8191:8191 ghcr.io/flaresolverr/flaresolverr:latest
```

**Step 2 — start the plugin:**

```bash
cd plugins/toonily
cp .env.example .env   # set FLARESOLVERR_URL=http://localhost:8191
npm install
npm run dev            # tsx watch mode
# or
npm run build && npm start
```

Plugin listens on `http://localhost:4002` by default.

**Step 3 — register with arrgh.** Pick one:

**Option A — auto-register at startup** (recommended for dev)

Add to `server/.env`:

```
PLUGIN_URLS=http://localhost:4002
```

Then (re)start the arrgh server. The URL is inserted once; subsequent restarts skip it.

**Option B — register at runtime**

With both arrgh and the plugin running, open **Settings → Sources → Add** and enter `http://localhost:4002`. No restart needed.

---

## Configuration

| Variable | Default | Description |
|---|---|---|
| `PORT` | `4002` | Listen port |
| `FLARESOLVERR_URL` | `http://flaresolverr:8191` | FlareSolverr base URL (required) |
| `API_KEY` | — | Require `Authorization: Bearer <key>` on all requests |

Copy `.env.example` to `.env` and edit as needed.

---

## Endpoints

| Method | Path | Description |
|---|---|---|
| `GET` | `/info` | Plugin metadata (id, name, content_types) |
| `GET` | `/search?q=` | Search Toonily by title |
| `GET` | `/trending` | Hot titles (Toonily trending ranking) |
| `GET` | `/manga/:slug/meta` | Description, cover, chapter count |
| `GET` | `/manga/:slug/chapters` | Full chapter list via Madara AJAX |
| `GET` | `/chapter/:manga_slug/:chapter_slug/pages` | Page image URLs |

---

## Notes

- Chapter `source_id` format: `{manga_slug}/{chapter_slug}` (e.g. `tower-of-god/chapter-578-0`)
- Chapter sync opens a FlareSolverr session (shared CF cookies across the manga detail GET and the `admin-ajax.php` POST) then destroys it
- FlareSolverr adds ~5–15 s latency per page fetch — expect chapter syncs to be slower than API-backed sources
- Selectors target the Madara WordPress theme; if Toonily updates their theme, selectors in `src/toonily.ts` may need adjustment
