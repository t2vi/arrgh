-- Port of EF migration NhentaiContentTypeHentai (data fixup, not schema).
-- Idempotent — the WHERE clause is a no-op once content_types is already 'hentai'.
UPDATE external_sources SET content_types = 'hentai' WHERE source_key = 'nhentai' AND content_types = 'manga';
