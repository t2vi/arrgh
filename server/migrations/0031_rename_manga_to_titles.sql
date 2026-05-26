-- Rename manga entity to title across all tables

ALTER TABLE manga RENAME TO titles;
ALTER TABLE manga_sources RENAME TO title_sources;
ALTER TABLE user_manga RENAME TO user_titles;
ALTER TABLE user_manga_settings RENAME TO user_title_settings;

ALTER TABLE chapters RENAME COLUMN manga_id TO title_id;
ALTER TABLE title_sources RENAME COLUMN manga_id TO title_id;
ALTER TABLE user_titles RENAME COLUMN manga_id TO title_id;
ALTER TABLE user_title_settings RENAME COLUMN manga_id TO title_id;

DROP INDEX IF EXISTS idx_chapters_manga_id;
CREATE INDEX IF NOT EXISTS idx_chapters_title_id ON chapters(title_id);

DROP INDEX IF EXISTS idx_manga_mangaupdates_id;
CREATE UNIQUE INDEX IF NOT EXISTS idx_titles_mangaupdates_id ON titles(mangaupdates_id) WHERE mangaupdates_id IS NOT NULL;

DROP INDEX IF EXISTS idx_user_manga_user;
CREATE INDEX IF NOT EXISTS idx_user_titles_user ON user_titles(user_id);
