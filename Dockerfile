# ── Stage 0: Build the Rust API server (ADR 0033 — strangler-fig on :3001) ────
FROM rust:1-slim AS rust-server-builder

WORKDIR /build
COPY server/Cargo.toml server/Cargo.lock ./
COPY server/src ./src
RUN cargo build --release --bin arrgh-server

# ── Stage 1: Build the .NET API server ───────────────────────────────────────
FROM mcr.microsoft.com/dotnet/sdk:10.0 AS server-builder

WORKDIR /build
COPY server/ .
RUN dotnet publish -c Release -o /publish --no-self-contained

# ── Stage 2: Build the React web app ─────────────────────────────────────────
FROM node:22-slim AS web-builder

WORKDIR /build/web
COPY web/package.json web/package-lock.json ./
RUN npm ci
COPY web/ ./
RUN npm run build

# ── Stage 3: Final image — nginx + .NET runtime ───────────────────────────────
FROM mcr.microsoft.com/dotnet/aspnet:10.0

RUN apt-get update && apt-get install -y nginx && rm -rf /var/lib/apt/lists/*

# nginx: serve web on :8080, proxy /api/* to .NET on :3000
COPY docker/nginx.conf /etc/nginx/sites-available/default

# .NET server publish output
COPY --from=server-builder /publish /app

# Rust server binary (ADR 0033) — runs alongside .NET during the migration
COPY --from=rust-server-builder /build/target/release/arrgh-server /app/arrgh-server

# Bundled plugin index (default when PluginIndexUrl not overridden)
COPY plugin-index/index.json /app/plugin-index.json

# Web assets
COPY --from=web-builder /build/web/dist /var/www/arrgh

# Startup: launch .NET server + nginx
COPY docker/entrypoint.sh /entrypoint.sh
RUN chmod +x /entrypoint.sh

VOLUME ["/data"]
EXPOSE 8080

ENTRYPOINT ["/entrypoint.sh"]
