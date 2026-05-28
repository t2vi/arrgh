CREATE TABLE IF NOT EXISTS sync_log (
    id         TEXT NOT NULL PRIMARY KEY,
    title_id   TEXT NOT NULL REFERENCES titles(id) ON DELETE CASCADE,
    message    TEXT NOT NULL,
    created_at TEXT NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_sync_log_title_id ON sync_log(title_id, created_at);
