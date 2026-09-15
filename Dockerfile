# ── Stage 0: Build the Rust API server (ADR 0033 — sole backend as of S10) ────
FROM rust:1-slim-bookworm AS rust-server-builder

WORKDIR /build
COPY server/Cargo.toml server/Cargo.lock ./
COPY server/src ./src
COPY server/migrations ./migrations
RUN cargo build --release --bin arrgh-server

# ── Stage 1: Build the Svelte web app (ADR 0033 — sole frontend as of F6) ─────
FROM node:22-slim AS web-builder

WORKDIR /build/web-svelte
COPY web-svelte/package.json web-svelte/package-lock.json ./
RUN npm ci
COPY web-svelte/ ./
RUN npm run build

# ── Stage 2: Final image — nginx + the Rust binary ────────────────────────────
# debian:bookworm-slim to match rust:1-slim-bookworm's glibc — the binary is
# dynamically linked (rustls avoids needing OpenSSL, but not libc).
FROM debian:bookworm-slim

RUN apt-get update && apt-get install -y nginx ca-certificates && rm -rf /var/lib/apt/lists/*

# nginx: serve web on :8080, proxy /api/* to the Rust server
COPY docker/nginx.conf /etc/nginx/sites-available/default

# Rust server binary (ADR 0033)
COPY --from=rust-server-builder /build/target/release/arrgh-server /app/arrgh-server

# Bundled plugin index (default when PluginIndexUrl not overridden)
COPY plugin-index/index.json /app/plugin-index.json

# Web assets
COPY --from=web-builder /build/web-svelte/dist /var/www/arrgh

# Startup: launch the Rust server + nginx
COPY docker/entrypoint.sh /entrypoint.sh
RUN chmod +x /entrypoint.sh

VOLUME ["/data"]
EXPOSE 8080

ENTRYPOINT ["/entrypoint.sh"]
