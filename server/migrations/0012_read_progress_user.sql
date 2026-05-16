-- SQLite can't ALTER UNIQUE constraints, so rebuild the table.
-- Existing rows get assigned the first user (or orphaned if no users exist — acceptable for fresh installs).

CREATE TABLE read_progress_new (
    id           TEXT PRIMARY KEY,
    user_id      TEXT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    chapter_id   TEXT NOT NULL REFERENCES chapters(id) ON DELETE CASCADE,
    current_page INTEGER NOT NULL DEFAULT 0,
    completed    INTEGER NOT NULL DEFAULT 0,
    updated_at   TEXT NOT NULL,
    UNIQUE(user_id, chapter_id)
);

INSERT INTO read_progress_new (id, user_id, chapter_id, current_page, completed, updated_at)
SELECT
    rp.id,
    COALESCE((SELECT id FROM users ORDER BY created_at LIMIT 1), 'unknown'),
    rp.chapter_id,
    rp.current_page,
    rp.completed,
    rp.updated_at
FROM read_progress rp;

DROP TABLE read_progress;
ALTER TABLE read_progress_new RENAME TO read_progress;

CREATE INDEX IF NOT EXISTS idx_read_progress_user ON read_progress(user_id);
CREATE INDEX IF NOT EXISTS idx_read_progress_chapter ON read_progress(chapter_id);
