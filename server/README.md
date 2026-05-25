# arrgh — server

Rust/Axum API server. Handles manga metadata, chapter downloads, source scraping, and read progress.

## Dev setup

```bash
cd server
cp .env.example .env   # edit DATABASE_URL, DOWNLOAD_DIR as needed
cargo run
```

API at `http://localhost:3000`. Docs at `http://localhost:3000/api/docs`.

## Environment variables

| Variable | Default | Description |
|---|---|---|
| `DATABASE_URL` | `sqlite://arrgh.db` | SQLite path |
| `DOWNLOAD_DIR` | `./downloads` | Chapter + cover download directory |
| `BIND_ADDR` | `0.0.0.0:3000` | Listen address |
| `INDEX_INTERVAL_HOURS` | `6` | Library sync interval |
| `PLUGIN_URLS` | — | Comma-separated plugin base URLs, auto-registered on startup (e.g. `http://localhost:4000,http://localhost:4001`) |
| `RUST_LOG` | — | Log level (`arrgh_server=debug`) |

## Database migrations

Always use `sqlx migrate` — never apply SQL directly:

```bash
sqlx migrate run --database-url sqlite://arrgh.db

# Create a new migration
sqlx migrate add <name>
```

Direct `sqlite3` execution bypasses the `_sqlx_migrations` table and causes duplicate-column errors on next startup.

## Tests

```bash
cargo test
```

## Project structure

```
src/
├── api/            # Route handlers
├── db/             # SQLx models
├── downloader/     # Background download worker
├── indexer/        # Source scrapers + sync scheduler
├── mangaupdates.rs # MangaUpdates API client (search, series detail, latest releases)
└── media/          # Image serving helpers
migrations/         # SQLx migration files
```
