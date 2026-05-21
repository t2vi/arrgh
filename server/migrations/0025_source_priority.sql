ALTER TABLE external_sources ADD COLUMN priority INTEGER NOT NULL DEFAULT 100;
ALTER TABLE external_sources ADD COLUMN source_key TEXT;
