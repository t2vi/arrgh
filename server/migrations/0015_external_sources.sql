CREATE TABLE IF NOT EXISTS external_sources (
    id           TEXT PRIMARY KEY,
    name         TEXT NOT NULL,
    base_url     TEXT NOT NULL,
    api_key      TEXT,
    content_types TEXT NOT NULL DEFAULT 'manga',
    enabled      INTEGER NOT NULL DEFAULT 1,
    created_at   TEXT NOT NULL
);
