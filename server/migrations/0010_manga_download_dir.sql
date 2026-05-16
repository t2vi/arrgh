ALTER TABLE manga ADD COLUMN download_dir TEXT DEFAULT NULL; -- NULL = use global _downloads/{title}
