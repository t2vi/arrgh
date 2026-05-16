CREATE TABLE IF NOT EXISTS manga (
    id          TEXT PRIMARY KEY,
    title       TEXT NOT NULL,
    description TEXT,
    cover_url   TEXT,
    status      TEXT NOT NULL DEFAULT 'unknown',
    source      TEXT NOT NULL DEFAULT 'local',
    source_id   TEXT,
    local_path  TEXT UNIQUE,
    created_at  TEXT NOT NULL,
    updated_at  TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS chapters (
    id          TEXT PRIMARY KEY,
    manga_id    TEXT NOT NULL REFERENCES manga(id) ON DELETE CASCADE,
    title       TEXT,
    number      REAL NOT NULL,
    volume      REAL,
    source_id   TEXT,
    local_path  TEXT,
    page_count  INTEGER NOT NULL DEFAULT 0,
    downloaded  INTEGER NOT NULL DEFAULT 0,
    created_at  TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS read_progress (
    id           TEXT PRIMARY KEY,
    chapter_id   TEXT NOT NULL UNIQUE REFERENCES chapters(id) ON DELETE CASCADE,
    current_page INTEGER NOT NULL DEFAULT 0,
    completed    INTEGER NOT NULL DEFAULT 0,
    updated_at   TEXT NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_chapters_manga_id ON chapters(manga_id);
CREATE INDEX IF NOT EXISTS idx_manga_source ON manga(source, source_id);
