# Docker Compose Deployment

## Quick start

```bash
curl -O https://raw.githubusercontent.com/t2vi/arrgh/main/docker-compose.yml
docker compose up -d
```

Open `http://<your-server-ip>:8282`. The setup wizard runs on first launch.

> ⚠️ **Port change (v0.1.3+)** — host port is **8282** (was 8080 in v0.1.2 and earlier). Update firewall rules, bookmarks, and reverse proxy configs accordingly.

---

## Services

| Service | Port (host) | Description |
|---|---|---|
| `arrgh` | 8282 | Main server + web UI (.NET 10 API + nginx) |
| `plugin-host` | _(internal)_ | Node.js plugin host — serves all bundled sources on port 4000 |
| `cloakbrowser` | _(internal)_ | Stealth Chromium CDP server for CF-protected sources |

`plugin-host` and `cloakbrowser` are internal only — not exposed to the host.

---

## Environment variables

### `arrgh`

| Variable | Default | Description |
|---|---|---|
| `DATABASE_URL` | `sqlite:///data/arrgh.db` | SQLite DB path inside the container |
| `DOWNLOAD_DIR` | `/data/downloads` | Where downloaded chapters are stored (must be inside the volume) |
| `JWT_SECRET` | _(random on startup)_ | Set this in production — sessions break on restart without it |
| `INDEX_INTERVAL_HOURS` | `6` | How often the background indexer runs |
| `PLUGIN_URLS` | `http://plugin-host:4000` | Comma-separated plugin URLs to auto-register on first boot |
| `PLUGIN_HOST_URL` | `http://plugin-host:4000` | Plugin host base URL (used by install/delete endpoints) |
| `PLUGIN_INDEX_URL` | `file:///app/plugin-index.json` | Plugin index for the Browse UI — bundled in image, override to use a remote index |
| `LOG_LEVEL` | `info` | Console + in-app log verbosity (`debug`/`info`/`warn`/`error`) |

### `plugin-host`

| Variable | Default | Description |
|---|---|---|
| `PORT` | `4000` | Plugin host listen port |
| `LANGUAGES` | `en` | Comma-separated language codes for chapter filtering |
| `CLOAKBROWSER_WS_URL` | `http://cloakbrowser:3000` | CloakBrowser CDP endpoint for CF-protected plugins |
| `COMMUNITY_BUNDLES_DIR` | `/community-bundles` | Where user-installed plugin bundles are persisted |

### `cloakbrowser`

| Variable | Default | Description |
|---|---|---|
| `CDP_PORT` | `3000` | CDP WebSocket port exposed to other containers |

CloakBrowser uses Google's public DNS (`8.8.8.8 / 8.8.4.4`) so CF-protected source domains resolve correctly inside Docker's internal network.

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

The volume must contain both the SQLite DB and the downloads directory:
- DB: `/data/arrgh.db` (+ `-shm` and `-wal` WAL files)
- Downloads: `/data/downloads/`

**Reverse proxy** — put nginx or Caddy in front to get HTTPS. See [nginx.md](nginx.md).

---

## Running without CloakBrowser

If you don't need CF-protected sources (Toonily, Comick, NovelFull), you can omit `cloakbrowser` and remove the dependency:

```yaml
services:
  arrgh:
    image: ghcr.io/t2vi/arrgh:latest
    depends_on:
      plugin-host:
        condition: service_healthy
    # ...

  plugin-host:
    image: ghcr.io/t2vi/plugin-host:latest
    environment:
      PORT: "4000"
      LANGUAGES: "en"
      # CLOAKBROWSER_WS_URL not set — CF-dependent plugins will error gracefully
```

CF-protected plugins will return errors for those sources; all others work normally.

---

## Web assets (Portainer / remote deployments)

The web UI is **baked into the Docker image** — no `web/dist` directory is needed on the host. If you deploy via Portainer or `docker compose pull`, it works out of the box.

The `docker-compose.override.yml` (used automatically for local `docker compose up`) mounts `./web/dist` over the image copy so local source changes appear without a rebuild. This override file is ignored when deploying remotely.

---

## Updating

```bash
docker compose pull
docker compose up -d
```

Migrations run automatically on startup — no manual DB steps needed.

---

## Debugging plugins locally

Expose the plugin-host port temporarily for direct API testing:

```yaml
plugin-host:
  ports:
    - "4000:4000"
```

Then:
```bash
curl http://localhost:4000/plugins        # list loaded plugins
curl http://localhost:4000/mangadex/info  # plugin info
curl "http://localhost:4000/mangadex/search?q=berserk"
```
