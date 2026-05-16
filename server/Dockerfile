# ── builder ────────────────────────────────────────────────────────────────────
FROM rust:1.87-slim AS builder

RUN apt-get update && apt-get install -y pkg-config libssl-dev && rm -rf /var/lib/apt/lists/*

WORKDIR /app

# cache deps
COPY Cargo.toml Cargo.lock ./
RUN mkdir src && echo 'fn main(){}' > src/main.rs && \
    cargo build --release && \
    rm -rf src target/release/arrgh-server*

# real build
COPY . .
ENV SQLX_OFFLINE=true
RUN cargo build --release

# ── runtime ────────────────────────────────────────────────────────────────────
FROM debian:bookworm-slim

RUN apt-get update && apt-get install -y ca-certificates && rm -rf /var/lib/apt/lists/*

WORKDIR /app
COPY --from=builder /app/target/release/arrgh-server ./
COPY migrations ./migrations

ENV BIND_ADDR=0.0.0.0:3000
EXPOSE 3000

CMD ["./arrgh-server"]
