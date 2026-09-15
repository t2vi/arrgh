# Manual Deployment

## Prerequisites

- Rust toolchain — https://rustup.rs (or a prebuilt binary — see Build)
- Node.js 22+
- nginx (or any static file server)

## Build

```bash
# API server
cd server
cargo build --release
cp target/release/arrgh-server /opt/arrgh/arrgh-server

# Web UI
cd ../web-svelte
npm ci
npm run build
# output at web-svelte/dist/
```

## Server setup

Create `/etc/arrgh/env`:

```
DatabasePath=/var/lib/arrgh/arrgh.db
DownloadDir=/var/lib/arrgh/downloads
PluginHostUrl=http://localhost:4000
PluginIndexUrl=file:///opt/arrgh/plugin-index.json
JwtSecret=<generate with: openssl rand -base64 48>
RUST_BIND=127.0.0.1:3001
LOG_LEVEL=info
```

Create a systemd unit `/etc/systemd/system/arrgh.service`:

```ini
[Unit]
Description=*ARRgh manga server
After=network.target

[Service]
User=arrgh
EnvironmentFile=/etc/arrgh/env
ExecStart=/opt/arrgh/arrgh-server
Restart=on-failure
StateDirectory=arrgh

[Install]
WantedBy=multi-user.target
```

```bash
useradd -r -s /sbin/nologin arrgh
cp plugin-index/index.json /opt/arrgh/plugin-index.json
systemctl enable --now arrgh
```

The server creates and migrates its own SQLite schema on first boot (`server/migrations/`, run via `sqlx migrate` — see `CLAUDE.md`'s Database Migrations section) and seeds the 9 bundled sources (MangaDex, Mangapill, Toonily, NovelFull, nhentai, MangaFire, Manga18fx, WuxiaWorld, AsuraScans) pointed at `PluginHostUrl` — no separate DB setup step, and no action needed to register the bundled sources. Set `SeedDefaultSources=false` to skip that seed (e.g. a DB restored from another instance that already has sources).

## Source plugins

*ARRgh ships with bundled sources served by `plugin-host`, auto-registered on first boot (see above). To add a community plugin, either:

**Option A — via the UI** (recommended)

Settings → Sources → Install a plugin from the index. Calls `POST /api/plugins/install`, which fetches the plugin bundle through `plugin-host` and registers it — no restart needed.

**Option B — register a source directly**

1. Start plugin-host: `cd plugin-host && npm start`
2. Open *ARRgh → Settings → Sources → Add*
3. Enter the plugin's base URL (e.g. `http://localhost:4000`)

---

## nginx for the web UI

```nginx
server {
    listen 80;
    server_name _;
    root /var/www/arrgh;
    index index.html;

    location /api/ {
        proxy_pass http://127.0.0.1:3001;
        proxy_read_timeout 300s;
    }

    location / {
        try_files $uri $uri/ /index.html;
    }
}
```

```bash
cp -r web-svelte/dist /var/www/arrgh
nginx -t && systemctl reload nginx
```
