CREATE UNIQUE INDEX IF NOT EXISTS idx_manga_source_unique
    ON manga(source, source_id) WHERE source_id IS NOT NULL;

CREATE UNIQUE INDEX IF NOT EXISTS idx_chapters_source_unique
    ON chapters(source_id) WHERE source_id IS NOT NULL;

CREATE TABLE IF NOT EXISTS download_queue (
    id           TEXT PRIMARY KEY,
    chapter_id   TEXT NOT NULL UNIQUE REFERENCES chapters(id) ON DELETE CASCADE,
    manga_title  TEXT NOT NULL,
    chapter_num  REAL NOT NULL,
    status       TEXT NOT NULL DEFAULT 'pending',
    error        TEXT,
    created_at   TEXT NOT NULL,
    updated_at   TEXT NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_queue_status ON download_queue(status, created_at);
