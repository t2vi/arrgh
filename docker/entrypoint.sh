#!/bin/sh
set -e

# ── Map friendly/legacy env vars to the config keys src/config.rs reads ──────
# (PascalCase names predate the Rust rewrite but are kept as the public
# contract — self-hosted users' existing env files keep working unchanged.)
# DATABASE_URL=sqlite:///data/arrgh.db → DatabasePath=/data/arrgh.db
if [ -n "$DATABASE_URL" ] && [ -z "$DatabasePath" ]; then
  export DatabasePath="${DATABASE_URL#sqlite:///}"
fi
export DatabasePath="${DatabasePath:-/data/arrgh.db}"

# PLUGIN_URLS → PluginHostUrl
if [ -n "$PLUGIN_URLS" ] && [ -z "$PluginHostUrl" ]; then
  export PluginHostUrl="$PLUGIN_URLS"
fi
export PluginHostUrl="${PluginHostUrl:-http://plugin-host:4000}"

# DOWNLOAD_DIR → DownloadDir
if [ -n "$DOWNLOAD_DIR" ] && [ -z "$DownloadDir" ]; then
  export DownloadDir="$DOWNLOAD_DIR"
fi
export DownloadDir="${DownloadDir:-/data/downloads}"

# JWT_SECRET → JwtSecret
# Auto-generate if not set — tokens invalidate on restart (set JWT_SECRET to persist)
if [ -n "$JWT_SECRET" ] && [ -z "$JwtSecret" ]; then
  export JwtSecret="$JWT_SECRET"
fi
if [ -z "$JwtSecret" ]; then
  export JwtSecret="$(cat /dev/urandom | tr -dc 'a-zA-Z0-9' | head -c 48)"
fi

export PluginIndexUrl="${PluginIndexUrl:-file:///app/plugin-index.json}"

# LOG_LEVEL=debug|info|warn|error (default: info) — controls both the
# docker console output and the in-app log viewer; read directly by the
# Rust binary, no translation needed.

mkdir -p "$DownloadDir"

# Start the Rust API server in background (ADR 0033 — sole backend as of
# S10 #132; reads DatabasePath/PluginHostUrl/DownloadDir/JwtSecret/
# PluginIndexUrl exported above).
RUST_BIND="127.0.0.1:3001" LOG_LEVEL="${LOG_LEVEL:-info}" /app/arrgh-server &

# Start nginx in foreground (keeps the container alive)
nginx -g "daemon off;"
