CREATE TABLE title_aliases (
    id TEXT PRIMARY KEY NOT NULL,
    title_id TEXT NOT NULL REFERENCES titles(id) ON DELETE CASCADE,
    alias TEXT NOT NULL
);
CREATE INDEX idx_title_aliases_title_id ON title_aliases(title_id);
