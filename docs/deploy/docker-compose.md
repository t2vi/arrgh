# Docker Compose Deployment

## Quick start

```bash
curl -O https://raw.githubusercontent.com/t2vi/arrgh/main/docker-compose.yml
docker compose up -d
```

Open `http://<your-server-ip>:8080`. The setup wizard runs on first launch.

---

## Services

| Service | Port (internal) | Description |
|---|---|---|
| `arrgh` | 8080 | Main server + web UI |
| `mangadex` | 4000 | MangaDex source plugin |
| `toonily` | 4001 | Toonily source plugin |
| `comick` | 4002 | Comick source plugin |
| `flaresolverr` | 8191 | Cloudflare bypass (used by Toonily + Comick) |

Plugin ports are internal only — not exposed to the host unless you add a `ports:` entry for debugging.

---

## Environment variables

### `arrgh`

| Variable | Default | Description |
|---|---|---|
| `DATABASE_URL` | `sqlite:///data/arrgh.db` | SQLite DB path inside the container |
| `DOWNLOAD_DIR` | `/data/manga` | Where downloaded chapters are stored |
| `JWT_SECRET` | _(random on startup)_ | Set this in production or sessions break on restart |
| `INDEX_INTERVAL_HOURS` | `6` | How often the background indexer runs |
| `PLUGIN_URLS` | see compose file | Comma-separated plugin URLs to auto-register on first boot |
| `RUST_LOG` | `arrgh_server=info` | Log level |
| `BIND_ADDR` | `0.0.0.0:8080` | Listen address inside the container |

### `mangadex`

| Variable | Default | Description |
|---|---|---|
| `PORT` | `4000` | Plugin listen port |
| `LANGUAGES` | `en` | Comma-separated language codes for chapter filtering |
| `API_KEY` | _(none)_ | If set, arrgh must send `Authorization: Bearer <key>` |

### `toonily`

| Variable | Default | Description |
|---|---|---|
| `PORT` | `4001` | Plugin listen port |
| `FLARESOLVERR_URL` | `http://flaresolverr:8191` | FlareSolverr base URL |
| `API_KEY` | _(none)_ | Optional plugin auth key |

### `comick`

| Variable | Default | Description |
|---|---|---|
| `PORT` | `4002` | Plugin listen port |
| `LANGUAGES` | `en` | Comma-separated language codes for chapter filtering |
| `FLARESOLVERR_URL` | `http://flaresolverr:8191` | FlareSolverr base URL |
| `API_KEY` | _(none)_ | Optional plugin auth key |

### `flaresolverr`

| Variable | Default | Description |
|---|---|---|
| `LOG_LEVEL` | `info` | FlareSolverr log verbosity (`debug`, `info`, `warning`, `error`) |

---

## Production checklist

**Set `JWT_SECRET`** — without it, every server restart invalidates all sessions:

```yaml
arrgh:
  environment:
    JWT_SECRET: "a-long-random-string-change-this"
```

**Persistent volume** — the default compose file uses a named volume (`arrgh_data`). To use a bind mount instead (easier to backup):

```yaml
volumes:
  - /your/path/arrgh:/data
```

**Reverse proxy** — put nginx or Caddy in front to get HTTPS. See [nginx.md](nginx.md).

**Plugin auth** — if your plugins are reachable from outside the Docker network, set `API_KEY` on each plugin and add the matching key when registering the source in Settings → Sources.

---

## Running only some plugins

To run arrgh without Toonily and Comick (no FlareSolverr needed):

```yaml
services:
  arrgh:
    environment:
      PLUGIN_URLS: "http://mangadex:4000"
    depends_on:
      - mangadex

  mangadex:
    build: ./plugins/mangadex
    environment:
      PORT: "4000"
      LANGUAGES: "en"

volumes:
  arrgh_data:
```

---

## Updating

```bash
docker compose pull
docker compose up -d
```

Migrations run automatically on startup — no manual DB steps needed.

---

## Debugging plugins locally

Expose a plugin port temporarily for direct API testing:

```yaml
toonily:
  ports:
    - "4001:4001"
```

Then `curl http://localhost:4001/info` to verify it's responding.
