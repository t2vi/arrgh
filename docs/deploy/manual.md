# Manual Deployment

## Prerequisites

- Rust (latest stable) — `curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh`
- Node.js 22+
- nginx (or any static file server)

## Build

```bash
# API server
cd server
cargo build --release
# binary at server/target/release/arrgh-server

# Web UI
cd ../web
npm ci
npm run build
# output at web/dist/
```

## Server setup

Create `/etc/arrgh/env`:

```
DATABASE_URL=sqlite:///var/lib/arrgh/arrgh.db
DOWNLOAD_DIR=/var/lib/arrgh/downloads
BIND_ADDR=127.0.0.1:3000
INDEX_INTERVAL_HOURS=6
```

Create a systemd unit `/etc/systemd/system/arrgh.service`:

```ini
[Unit]
Description=*ARRgh manga server
After=network.target

[Service]
User=arrgh
EnvironmentFile=/etc/arrgh/env
ExecStart=/usr/local/bin/arrgh-server
Restart=on-failure
StateDirectory=arrgh

[Install]
WantedBy=multi-user.target
```

```bash
useradd -r -s /sbin/nologin arrgh
cp server/target/release/arrgh-server /usr/local/bin/
systemctl enable --now arrgh
```

## Source plugins

*ARRgh ships with Mangapill as a built-in source. To add external sources (e.g. the MangaDex plugin), you need to register them either at startup or at runtime.

**Option A — auto-register at startup** (recommended)

Add to `/etc/arrgh/env`:

```
PLUGIN_URLS=http://localhost:4000
```

Restart the server. Each URL in `PLUGIN_URLS` is probed on boot and inserted into the DB if not already present — idempotent across restarts.

**Option B — register at runtime via the UI**

1. Start your plugin (e.g. `cd plugins/mangadex && npm start`)
2. Open *ARRgh → Settings → Sources → Add*
3. Enter the plugin's base URL (e.g. `http://localhost:4000`)

No server restart needed — the registry hot-reloads immediately.

---

## nginx for the web UI

```nginx
server {
    listen 80;
    server_name _;
    root /var/www/arrgh;
    index index.html;

    location /api/ {
        proxy_pass http://127.0.0.1:3000;
        proxy_read_timeout 300s;
    }

    location / {
        try_files $uri $uri/ /index.html;
    }
}
```

```bash
cp -r web/dist /var/www/arrgh
nginx -t && systemctl reload nginx
```
