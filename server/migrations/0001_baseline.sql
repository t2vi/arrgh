-- Baseline schema (ADR 0033, S10 #132). Reproduces the schema EF Core's 4
-- migrations produced (InitialSchema + AddMetadataSourceColumns +
-- NhentaiContentTypeHentai + RemoveBrokenPlugins — the latter two are
-- one-time data fixups, not schema, and live in 0002/0003).
--
-- Every statement is IF NOT EXISTS: an existing .NET-managed database
-- already has this exact schema, so on the first Rust-only boot this file
-- must be a safe no-op against it. `src/state.rs`'s `bootstrap_schema`
-- stamps `_sqlx_migrations` for such a database before the migrator even
-- runs — same trick ADR 0030 used for `__EFMigrationsHistory` — but the
-- IF NOT EXISTS is the real safety net if that stamp is ever skipped.

CREATE TABLE IF NOT EXISTS "external_sources" (
    "id" TEXT NOT NULL PRIMARY KEY,
    "name" TEXT NOT NULL,
    "base_url" TEXT NOT NULL,
    "api_key" TEXT,
    "content_types" TEXT NOT NULL,
    "enabled" INTEGER NOT NULL,
    "created_at" TEXT NOT NULL,
    "is_community" INTEGER NOT NULL,
    "priority" INTEGER NOT NULL,
    "source_key" TEXT,
    "default_explicit" INTEGER NOT NULL
);

CREATE TABLE IF NOT EXISTS "server_settings" (
    "key" TEXT NOT NULL PRIMARY KEY,
    "value" TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS "title_meta" (
    "title_key" TEXT NOT NULL PRIMARY KEY,
    "cover_local_path" TEXT,
    "cover_cdn_url" TEXT,
    "description" TEXT,
    "tags" TEXT,
    "chapter_count" INTEGER NOT NULL,
    "source" TEXT NOT NULL,
    "source_id" TEXT NOT NULL,
    "fetched_at" TEXT NOT NULL
);

-- metadata_source/metadata_source_id folded in directly (EF added them via
-- a later migration; there's no "before" state worth reproducing here).
CREATE TABLE IF NOT EXISTS "titles" (
    "id" TEXT NOT NULL PRIMARY KEY,
    "title" TEXT NOT NULL,
    "description" TEXT,
    "cover_url" TEXT,
    "status" TEXT NOT NULL,
    "created_at" TEXT NOT NULL,
    "updated_at" TEXT NOT NULL,
    "author" TEXT,
    "year" INTEGER,
    "tags" TEXT,
    "sync_status" TEXT NOT NULL,
    "content_type" TEXT NOT NULL,
    "auto_download" INTEGER,
    "reader_mode" TEXT,
    "download_dir" TEXT,
    "is_explicit" INTEGER NOT NULL DEFAULT 0,
    "mangaupdates_id" TEXT,
    "local_path" TEXT,
    "metadata_source" TEXT,
    "metadata_source_id" TEXT
);
CREATE UNIQUE INDEX IF NOT EXISTS "IX_titles_mangaupdates_id" ON "titles" ("mangaupdates_id") WHERE mangaupdates_id IS NOT NULL;

CREATE TABLE IF NOT EXISTS "users" (
    "id" TEXT NOT NULL PRIMARY KEY,
    "username" TEXT NOT NULL,
    "password_hash" TEXT NOT NULL,
    "created_at" TEXT NOT NULL,
    "role" TEXT NOT NULL,
    "allow_explicit" INTEGER NOT NULL DEFAULT 0
);
CREATE UNIQUE INDEX IF NOT EXISTS "IX_users_username" ON "users" ("username");

CREATE TABLE IF NOT EXISTS "chapters" (
    "id" TEXT NOT NULL PRIMARY KEY,
    "title_id" TEXT NOT NULL REFERENCES "titles" ("id") ON DELETE CASCADE,
    "title" TEXT,
    "number" REAL NOT NULL,
    "volume" REAL,
    "local_path" TEXT,
    "page_count" INTEGER NOT NULL,
    "downloaded" INTEGER NOT NULL,
    "created_at" TEXT NOT NULL,
    "is_new" INTEGER NOT NULL,
    "chapter_format" TEXT NOT NULL
);
CREATE INDEX IF NOT EXISTS "idx_chapters_title_id" ON "chapters" ("title_id");

CREATE TABLE IF NOT EXISTS "sync_log" (
    "id" TEXT NOT NULL PRIMARY KEY,
    "title_id" TEXT NOT NULL REFERENCES "titles" ("id") ON DELETE CASCADE,
    "message" TEXT NOT NULL,
    "created_at" TEXT NOT NULL
);
CREATE INDEX IF NOT EXISTS "idx_sync_log_title_id" ON "sync_log" ("title_id", "created_at");

CREATE TABLE IF NOT EXISTS "sync_warnings" (
    "id" TEXT NOT NULL PRIMARY KEY,
    "title_id" TEXT NOT NULL REFERENCES "titles" ("id") ON DELETE CASCADE,
    "plugin_id" TEXT NOT NULL,
    "message" TEXT NOT NULL,
    "created_at" TEXT NOT NULL
);
CREATE INDEX IF NOT EXISTS "idx_sync_warnings_title_id" ON "sync_warnings" ("title_id");
CREATE UNIQUE INDEX IF NOT EXISTS "IX_sync_warnings_title_id_plugin_id" ON "sync_warnings" ("title_id", "plugin_id");

CREATE TABLE IF NOT EXISTS "title_aliases" (
    "id" TEXT NOT NULL PRIMARY KEY,
    "title_id" TEXT NOT NULL REFERENCES "titles" ("id") ON DELETE CASCADE,
    "alias" TEXT NOT NULL
);
CREATE INDEX IF NOT EXISTS "idx_title_aliases_title_id" ON "title_aliases" ("title_id");

CREATE TABLE IF NOT EXISTS "title_sources" (
    "id" TEXT NOT NULL PRIMARY KEY,
    "title_id" TEXT NOT NULL REFERENCES "titles" ("id") ON DELETE CASCADE,
    "source" TEXT NOT NULL,
    "source_id" TEXT NOT NULL,
    "discovered_at" TEXT NOT NULL
);
CREATE UNIQUE INDEX IF NOT EXISTS "IX_title_sources_title_id_source" ON "title_sources" ("title_id", "source");

CREATE TABLE IF NOT EXISTS "user_title_settings" (
    "user_id" TEXT NOT NULL REFERENCES "users" ("id") ON DELETE CASCADE,
    "title_id" TEXT NOT NULL REFERENCES "titles" ("id") ON DELETE CASCADE,
    "reader_mode" TEXT,
    PRIMARY KEY ("user_id", "title_id")
);
CREATE INDEX IF NOT EXISTS "IX_user_title_settings_title_id" ON "user_title_settings" ("title_id");

CREATE TABLE IF NOT EXISTS "user_titles" (
    "user_id" TEXT NOT NULL REFERENCES "users" ("id") ON DELETE CASCADE,
    "title_id" TEXT NOT NULL REFERENCES "titles" ("id") ON DELETE CASCADE,
    "added_at" TEXT NOT NULL,
    PRIMARY KEY ("user_id", "title_id")
);
CREATE INDEX IF NOT EXISTS "idx_user_titles_user" ON "user_titles" ("user_id");
CREATE INDEX IF NOT EXISTS "IX_user_titles_title_id" ON "user_titles" ("title_id");

CREATE TABLE IF NOT EXISTS "chapter_sources" (
    "id" TEXT NOT NULL PRIMARY KEY,
    "chapter_id" TEXT NOT NULL REFERENCES "chapters" ("id") ON DELETE CASCADE,
    "source" TEXT NOT NULL,
    "source_id" TEXT NOT NULL
);
CREATE INDEX IF NOT EXISTS "idx_chapter_sources_chapter_id" ON "chapter_sources" ("chapter_id");
CREATE UNIQUE INDEX IF NOT EXISTS "IX_chapter_sources_chapter_id_source" ON "chapter_sources" ("chapter_id", "source");

CREATE TABLE IF NOT EXISTS "download_queue" (
    "id" TEXT NOT NULL PRIMARY KEY,
    "chapter_id" TEXT NOT NULL REFERENCES "chapters" ("id") ON DELETE CASCADE,
    "manga_title" TEXT NOT NULL,
    "chapter_num" REAL NOT NULL,
    "status" TEXT NOT NULL,
    "error" TEXT,
    "created_at" TEXT NOT NULL,
    "updated_at" TEXT NOT NULL,
    "pages_downloaded" INTEGER NOT NULL,
    "pages_total" INTEGER NOT NULL,
    "queued_by" TEXT REFERENCES "users" ("id") ON DELETE SET NULL
);
CREATE INDEX IF NOT EXISTS "idx_queue_status" ON "download_queue" ("status", "created_at");
CREATE UNIQUE INDEX IF NOT EXISTS "IX_download_queue_chapter_id" ON "download_queue" ("chapter_id");
CREATE INDEX IF NOT EXISTS "IX_download_queue_queued_by" ON "download_queue" ("queued_by");

CREATE TABLE IF NOT EXISTS "read_progress" (
    "id" TEXT NOT NULL PRIMARY KEY,
    "user_id" TEXT NOT NULL REFERENCES "users" ("id") ON DELETE CASCADE,
    "chapter_id" TEXT NOT NULL REFERENCES "chapters" ("id") ON DELETE CASCADE,
    "current_page" INTEGER NOT NULL,
    "completed" INTEGER NOT NULL,
    "updated_at" TEXT NOT NULL
);
CREATE INDEX IF NOT EXISTS "idx_read_progress_chapter" ON "read_progress" ("chapter_id");
CREATE INDEX IF NOT EXISTS "idx_read_progress_user" ON "read_progress" ("user_id");
CREATE UNIQUE INDEX IF NOT EXISTS "IX_read_progress_user_id_chapter_id" ON "read_progress" ("user_id", "chapter_id");
