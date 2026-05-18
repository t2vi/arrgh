CREATE TABLE IF NOT EXISTS title_meta (
    title_key        TEXT PRIMARY KEY,
    cover_local_path TEXT,
    cover_cdn_url    TEXT,
    description      TEXT,
    tags             TEXT,
    chapter_count    INTEGER NOT NULL DEFAULT 0,
    source           TEXT NOT NULL,
    source_id        TEXT NOT NULL,
    fetched_at       TEXT NOT NULL
);
