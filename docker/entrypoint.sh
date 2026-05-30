#!/bin/sh
set -e

# ── Map legacy env vars to .NET config keys (backward compat) ────────────────
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

# LOG_LEVEL=debug|info|warn|error (default: info)
# Controls both the docker console output and the in-app log viewer.
case "${LOG_LEVEL:-info}" in
  debug) export Logging__LogLevel__Default="Debug" ;;
  warn)  export Logging__LogLevel__Default="Warning" ;;
  error) export Logging__LogLevel__Default="Error" ;;
  *)     export Logging__LogLevel__Default="Information" ;;
esac

export ASPNETCORE_URLS="http://127.0.0.1:3000"
export ASPNETCORE_ENVIRONMENT="${ASPNETCORE_ENVIRONMENT:-Production}"

mkdir -p "$DownloadDir"

# Start .NET API server in background
dotnet /app/ArrghServer.dll &

# Start nginx in foreground (keeps the container alive)
nginx -g "daemon off;"
