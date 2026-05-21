CREATE TABLE IF NOT EXISTS manga_sources (
    id           TEXT PRIMARY KEY,
    manga_id     TEXT NOT NULL REFERENCES manga(id) ON DELETE CASCADE,
    source       TEXT NOT NULL,
    source_id    TEXT NOT NULL,
    discovered_at TEXT NOT NULL,
    UNIQUE(manga_id, source)
);

INSERT OR IGNORE INTO manga_sources (id, manga_id, source, source_id, discovered_at)
SELECT lower(hex(randomblob(16))), id, source, source_id, created_at
FROM manga
WHERE source != 'local' AND source_id IS NOT NULL;
