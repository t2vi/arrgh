#!/usr/bin/env bash
# One-command local dev bring-up: Rust API server, Vite web server, plugin-host,
# and CloakBrowser. Ctrl-C stops all four. See CLAUDE.md's Dev Workflow for the
# per-terminal equivalent (restart one service without restarting everything).
set -u
cd "$(dirname "$0")/.."

CLOAKBROWSER_PORT=3002
CLOAKBROWSER_NAME=arrgh-cloakbrowser-1

CONTAINER_CMD=""
for c in docker podman; do
  command -v "$c" >/dev/null 2>&1 && { CONTAINER_CMD=$c; break; }
done
if [ -z "$CONTAINER_CMD" ]; then
  echo "error: neither docker nor podman found on PATH — required for CloakBrowser" >&2
  exit 1
fi

echo "[dev-up] container runtime: $CONTAINER_CMD"

if $CONTAINER_CMD ps --format '{{.Names}}' 2>/dev/null | grep -qx "$CLOAKBROWSER_NAME"; then
  echo "[dev-up] cloakbrowser: already running"
elif $CONTAINER_CMD ps -a --format '{{.Names}}' 2>/dev/null | grep -qx "$CLOAKBROWSER_NAME"; then
  echo "[dev-up] cloakbrowser: starting existing container"
  $CONTAINER_CMD start "$CLOAKBROWSER_NAME" >/dev/null
else
  echo "[dev-up] cloakbrowser: building image (first run only, ~1-2 min)"
  # ponytail: no --platform pin (compose's linux/amd64 is for the ghcr production
  # image) — building for the host's native arch is faster locally and CDP/Chromium
  # automation works fine either way; add --platform linux/amd64 back if a plugin
  # ever needs arch-specific behavior.
  $CONTAINER_CMD build -t arrgh-cloakbrowser docker/cloakbrowser-server || exit 1
  $CONTAINER_CMD run -d --name "$CLOAKBROWSER_NAME" -p "${CLOAKBROWSER_PORT}:3000" arrgh-cloakbrowser >/dev/null
fi

pids=()
start() {
  ( "$@" ) &
  pids+=("$!")
}

start bash -c "cd server && JwtSecret=dev-secret cargo run"
start bash -c "cd web-svelte && npm run dev"
start bash -c "cd plugin-host && CLOAKBROWSER_WS_URL=http://localhost:${CLOAKBROWSER_PORT} npm start"

cleanup() {
  echo ""
  echo "[dev-up] stopping…"
  for pid in "${pids[@]}"; do
    pkill -P "$pid" 2>/dev/null   # each backgrounded process's own children (cargo's server binary, vite, node)
    kill "$pid" 2>/dev/null
  done
  $CONTAINER_CMD stop "$CLOAKBROWSER_NAME" >/dev/null 2>&1
}
trap cleanup EXIT INT TERM

echo "[dev-up] all services starting — web http://localhost:5173, api http://localhost:3001, plugin-host http://localhost:4000, cloakbrowser http://localhost:${CLOAKBROWSER_PORT}"
echo "[dev-up] Ctrl-C to stop everything"
wait
