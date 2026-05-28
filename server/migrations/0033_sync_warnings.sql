CREATE TABLE sync_warnings (
    id TEXT PRIMARY KEY NOT NULL,
    title_id TEXT NOT NULL REFERENCES titles(id) ON DELETE CASCADE,
    plugin_id TEXT NOT NULL,
    message TEXT NOT NULL,
    created_at TEXT NOT NULL,
    UNIQUE(title_id, plugin_id)
);
CREATE INDEX idx_sync_warnings_title_id ON sync_warnings(title_id);
