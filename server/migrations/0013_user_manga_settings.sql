CREATE TABLE IF NOT EXISTS user_manga_settings (
    user_id    TEXT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    manga_id   TEXT NOT NULL REFERENCES manga(id) ON DELETE CASCADE,
    reader_mode TEXT,
    PRIMARY KEY (user_id, manga_id)
);

-- Migrate existing global reader_mode into settings for first user
INSERT INTO user_manga_settings (user_id, manga_id, reader_mode)
SELECT
    (SELECT id FROM users ORDER BY created_at LIMIT 1),
    m.id,
    m.reader_mode
FROM manga m
WHERE m.reader_mode IS NOT NULL
  AND (SELECT id FROM users ORDER BY created_at LIMIT 1) IS NOT NULL;
