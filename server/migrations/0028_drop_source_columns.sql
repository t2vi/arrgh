DROP INDEX IF EXISTS idx_manga_source;
DROP INDEX IF EXISTS idx_manga_source_unique;
DROP INDEX IF EXISTS idx_chapters_source_unique;
ALTER TABLE manga DROP COLUMN source;
ALTER TABLE manga DROP COLUMN source_id;
ALTER TABLE chapters DROP COLUMN source_id;
