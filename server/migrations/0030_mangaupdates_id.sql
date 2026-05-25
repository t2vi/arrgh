ALTER TABLE manga ADD COLUMN mangaupdates_id TEXT;
CREATE UNIQUE INDEX IF NOT EXISTS idx_manga_mangaupdates_id ON manga(mangaupdates_id) WHERE mangaupdates_id IS NOT NULL;
