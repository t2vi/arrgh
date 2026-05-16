CREATE TABLE IF NOT EXISTS user_manga (
    user_id    TEXT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    manga_id   TEXT NOT NULL REFERENCES manga(id) ON DELETE CASCADE,
    added_at   TEXT NOT NULL,
    PRIMARY KEY (user_id, manga_id)
);

CREATE INDEX IF NOT EXISTS idx_user_manga_user ON user_manga(user_id);

-- Assign all existing manga to all existing users so nothing disappears on upgrade
INSERT OR IGNORE INTO user_manga (user_id, manga_id, added_at)
SELECT u.id, m.id, m.created_at
FROM users u
CROSS JOIN manga m;
