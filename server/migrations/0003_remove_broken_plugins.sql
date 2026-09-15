-- Port of EF migration RemoveBrokenPlugins (data fixup, not schema).
-- Idempotent — no-op once these rows are gone.
DELETE FROM external_sources WHERE source_key IN ('royalroad', 'manhuafast', 'boxnovel');
