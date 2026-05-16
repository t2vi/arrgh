#!/bin/sh
set -e

# Default env vars
export DATABASE_URL="${DATABASE_URL:-sqlite:///data/arrgh.db}"
export MANGA_DIR="${MANGA_DIR:-/data/manga}"
export BIND_ADDR="${BIND_ADDR:-127.0.0.1:3000}"

mkdir -p /data/manga

# Start Rust API server in background
arrgh-server &

# Start nginx in foreground (keeps the container alive)
nginx -g "daemon off;"
