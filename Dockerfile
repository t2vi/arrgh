# ── Stage 1: Build the Rust API server ───────────────────────────────────────
FROM rust:1-slim AS server-builder

RUN apt-get update && apt-get install -y pkg-config libssl-dev && rm -rf /var/lib/apt/lists/*

WORKDIR /build/server

ENV SQLX_OFFLINE=true

# Cache dependencies (needs .sqlx/ for compile-time query verification)
COPY server/Cargo.toml server/Cargo.lock ./
COPY server/.sqlx ./.sqlx
RUN mkdir src && echo "fn main() {}" > src/main.rs && cargo build --release && rm -rf src

COPY server/src ./src
COPY server/migrations ./migrations
RUN touch src/main.rs && cargo build --release

# ── Stage 2: Build the React web app ─────────────────────────────────────────
FROM node:22-slim AS web-builder

WORKDIR /build/web
COPY web/package.json web/package-lock.json ./
RUN npm ci
COPY web/ ./
RUN npm run build

# ── Stage 3: Final image — nginx + Rust binary ───────────────────────────────
FROM debian:bookworm-slim

RUN apt-get update && apt-get install -y ca-certificates nginx && rm -rf /var/lib/apt/lists/*

# Nginx config: serve web on :80, proxy /api/* to Rust on :3000
COPY docker/nginx.conf /etc/nginx/sites-available/default

# Rust binary
COPY --from=server-builder /build/server/target/release/arrgh-server /usr/local/bin/arrgh-server

# Bundled plugin index (used when PLUGIN_INDEX_URL is not set)
COPY plugin-index/index.json /app/plugin-index.json

# Web assets
COPY --from=web-builder /build/web/dist /var/www/arrgh

# Startup script: launch Rust server + nginx
COPY docker/entrypoint.sh /entrypoint.sh
RUN chmod +x /entrypoint.sh

VOLUME ["/data"]
EXPOSE 8080

ENTRYPOINT ["/entrypoint.sh"]
