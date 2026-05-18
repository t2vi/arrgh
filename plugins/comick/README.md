# Comick source plugin

*ARRgh plugin for [Comick](https://comick.fun) — manga, manhwa, and manhua aggregator.

## Requirements

Requires a [FlareSolverr](https://github.com/FlareSolverr/FlareSolverr) sidecar — Comick's API (`api.comick.dev`) is Cloudflare-protected.

```bash
docker run -d -p 8191:8191 ghcr.io/flaresolverr/flaresolverr:latest
```

## Dev setup

```bash
cp .env.example .env   # set FLARESOLVERR_URL=http://localhost:8191
npm install
npm run dev            # starts on :4003
```

## Environment variables

| Variable | Default | Description |
|---|---|---|
| `PORT` | `4003` | Port to listen on |
| `LANGUAGES` | `en` | Comma-separated language codes for chapter filtering |
| `FLARESOLVERR_URL` | `http://flaresolverr:8191` | FlareSolverr base URL |
| `API_KEY` | _(none)_ | If set, arrgh must send `Authorization: Bearer <key>` |

## Endpoints

| Endpoint | Notes |
|---|---|
| `GET /info` | `id: comick`, `content_types: [manga, manhwa, manhua]` |
| `GET /search?q=` | Searches Comick catalog |
| `GET /trending` | Most-followed titles |
| `GET /manga/:slug/meta` | Description, cover, chapter count, tags |
| `GET /manga/:slug/chapters` | All chapters, deduplicated (first scanlation per number), filtered by `LANGUAGES` |
| `GET /chapter/:hid/pages` | Page image URLs for a chapter |

## Explicit content

Comick exposes `content_rating` per title (`safe`, `suggestive`, `erotica`). Titles rated `erotica` are normalized to the `"adult"` tag, which arrgh uses to set `is_explicit = true` during sync. Users without Explicit Permission will not see these titles.
