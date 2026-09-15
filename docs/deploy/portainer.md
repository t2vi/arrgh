# Portainer Deployment

*ARRgh!* ships as standard Docker Compose services (see [docker-compose.md](docker-compose.md)), so Portainer can deploy it as a **stack** with no separate config format — same image, same env vars, same volumes.

## Method 1: Repository (recommended)

Lets Portainer auto-redeploy when the repo's `docker-compose.yml` changes (via a webhook), and keeps the stack definition out of Portainer's own storage.

1. **Stacks → Add stack**
2. **Build method: Repository**
3. **Repository URL**: `https://github.com/t2vi/arrgh`
4. **Compose path**: `docker-compose.yml`
5. **Deploy the stack**

## Method 2: Web editor

Paste the contents of [`docker-compose.yml`](../../docker-compose.yml) directly:

1. **Stacks → Add stack**
2. **Build method: Web editor**
3. Paste the file contents
4. **Deploy the stack**

Either method pulls `ghcr.io/t2vi/arrgh:latest` (and the `plugin-host`/`cloakbrowser` images) — no local build step, no `web-svelte/dist` needed on the host (the web UI is baked into the image).

---

## Setting environment variables

Portainer's stack view has its own **Environment variables** editor — use it instead of hand-editing the pasted Compose YAML:

**Stacks → arrgh → Environment variables → Add environment variable**

| Variable | Example | Notes |
|---|---|---|
| `DATABASE_URL` | `sqlite:///data/arrgh.db` | SQLite path inside the container |
| `DOWNLOAD_DIR` | `/data/downloads` | Must stay inside the mounted volume |
| `JWT_SECRET` | _(a long random string)_ | Set this — without it, sessions reset on every redeploy |
| `PLUGIN_URLS` | `http://plugin-host:4000` | Auto-registers the bundled plugins on first boot |
| `LOG_LEVEL` | `info` | `debug`/`info`/`warn`/`error` |

These are the same variables `docker-compose.yml` sets inline — see [docker-compose.md](docker-compose.md#environment-variables) for the full list, including `plugin-host` and `cloakbrowser` variables. (Their PascalCase equivalents — `DatabasePath`, `DownloadDir`, `PluginHostUrl`, `PluginIndexUrl`, `JwtSecret` — also work directly; the container entrypoint translates the friendly names above into these at boot.)

After changing a variable, **Update the stack** (Portainer's redeploy) to apply it.

---

## Persistent storage

Portainer creates the named volumes (`arrgh_data`, `plugin_community_bundles`) declared in the Compose file automatically. To inspect or back them up: **Volumes** in Portainer's sidebar, or `docker volume inspect arrgh_arrgh_data` from the host.

## Upgrading

**Stacks → arrgh → Pull and redeploy** (or re-trigger the repository webhook). Same behavior as `docker compose pull && docker compose up -d` — migrations run automatically on startup.

## Support level

CI-tested via the underlying `docker-compose.yml` (same image, same e2e stack). Portainer's own stack UI behavior itself isn't exercised by this project's CI.
