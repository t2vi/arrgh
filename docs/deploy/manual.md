# Manual Deployment

## Prerequisites

- .NET 10 SDK — https://dotnet.microsoft.com/download
- Node.js 22+
- nginx (or any static file server)

## Build

```bash
# API server
cd server
dotnet publish -c Release -o /opt/arrgh/server

# Web UI
cd ../web
npm ci
npm run build
# output at web/dist/
```

## Server setup

Create `/etc/arrgh/env`:

```
DatabasePath=/var/lib/arrgh/arrgh.db
DownloadDir=/var/lib/arrgh/downloads
PluginHostUrl=http://localhost:4000
PluginIndexUrl=file:///opt/arrgh/plugin-index.json
JwtSecret=<generate with: openssl rand -base64 48>
ASPNETCORE_URLS=http://127.0.0.1:3000
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
ExecStart=/usr/bin/dotnet /opt/arrgh/server/ArrghServer.dll
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

## Source plugins

*ARRgh ships with bundled sources (MangaDex, Mangapill, etc.) served by `plugin-host`. To register them, set `PLUGIN_URLS` to the plugin-host base URL.

**Option A — auto-register at startup** (recommended)

Add to `/etc/arrgh/env`:

```
PLUGIN_URLS=http://localhost:4000
```

Restart the server. Each URL in `PLUGIN_URLS` is probed on boot and inserted into the DB if not already present — idempotent across restarts.

**Option B — register at runtime via the UI**

1. Start plugin-host: `cd plugin-host && npm start`
2. Open *ARRgh → Settings → Sources → Add*
3. Enter the plugin's base URL (e.g. `http://localhost:4000`)

No server restart needed — the registry updates immediately.

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
