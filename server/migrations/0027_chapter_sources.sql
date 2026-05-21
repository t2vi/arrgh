CREATE TABLE IF NOT EXISTS chapter_sources (
    id         TEXT PRIMARY KEY,
    chapter_id TEXT NOT NULL REFERENCES chapters(id) ON DELETE CASCADE,
    source     TEXT NOT NULL,
    source_id  TEXT NOT NULL,
    UNIQUE(chapter_id, source)
);

CREATE INDEX IF NOT EXISTS idx_chapter_sources_chapter_id ON chapter_sources(chapter_id);

-- Backfill: each chapter inherits its manga's source
INSERT OR IGNORE INTO chapter_sources (id, chapter_id, source, source_id)
SELECT lower(hex(randomblob(16))), c.id, m.source, c.source_id
FROM chapters c
JOIN manga m ON m.id = c.manga_id
WHERE c.source_id IS NOT NULL AND m.source != 'local';
