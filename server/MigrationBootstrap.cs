using ArrghServer.Data;
using Microsoft.Data.Sqlite;
using Microsoft.EntityFrameworkCore;

namespace ArrghServer;

public static class MigrationBootstrap
{
    const string V = "10.0.8";

    // Called before db.Database.Migrate() to handle databases created before EF migrations were
    // introduced (Rust server era or pre-migration .NET build). If __EFMigrationsHistory is absent
    // but external_sources exists, we repair the schema (create missing tables, add missing columns)
    // and record the applied migrations so Migrate() only runs genuinely new ones.
    public static void Bootstrap(AppDbContext db)
    {
        db.Database.OpenConnection();
        try
        {
            var conn = (SqliteConnection)db.Database.GetDbConnection();

            using var check = conn.CreateCommand();

            // Already migrated — nothing to do
            check.CommandText = "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name='__EFMigrationsHistory'";
            if ((long)check.ExecuteScalar()! > 0) return;

            // Fresh DB — let Migrate() handle everything
            check.CommandText = "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name='external_sources'";
            if ((long)check.ExecuteScalar()! == 0) return;

            // Pre-migration DB detected: repair schema then record applied migrations
            CreateMissingTables(conn);
            PatchExternalSourcesColumns(conn);

            // Create history and record InitialSchema
            Exec(conn, $"""
                CREATE TABLE "__EFMigrationsHistory" (
                    "MigrationId" TEXT NOT NULL CONSTRAINT "PK___EFMigrationsHistory" PRIMARY KEY,
                    "ProductVersion" TEXT NOT NULL
                );
                INSERT INTO "__EFMigrationsHistory" VALUES ('20260529064158_InitialSchema', '{V}');
                """);

            // Record AddMetadataSourceColumns if already applied (column exists)
            check.CommandText = "SELECT COUNT(*) FROM pragma_table_info('titles') WHERE name='metadata_source'";
            if ((long)check.ExecuteScalar()! > 0)
                Exec(conn, $"INSERT INTO \"__EFMigrationsHistory\" VALUES ('20260529104924_AddMetadataSourceColumns', '{V}')");
        }
        finally
        {
            db.Database.CloseConnection();
        }
    }

    // Create all tables from InitialSchema that may not exist in old DBs
    static void CreateMissingTables(SqliteConnection conn)
    {
        // Execute as individual statements — SQLite ExecuteNonQuery only runs the first stmt
        foreach (var sql in InitialSchemaSql)
            Exec(conn, sql);
    }

    // Add columns that existed in the EF model but not in old Rust DB builds
    static void PatchExternalSourcesColumns(SqliteConnection conn)
    {
        EnsureColumn(conn, "external_sources", "is_community",    "INTEGER NOT NULL DEFAULT 0");
        EnsureColumn(conn, "external_sources", "source_key",      "TEXT");
        EnsureColumn(conn, "external_sources", "default_explicit", "INTEGER NOT NULL DEFAULT 0");
    }

    static void EnsureColumn(SqliteConnection conn, string table, string column, string def)
    {
        using var check = conn.CreateCommand();
        check.CommandText = $"SELECT COUNT(*) FROM pragma_table_info('{table}') WHERE name='{column}'";
        if ((long)check.ExecuteScalar()! > 0) return;
        Exec(conn, $"ALTER TABLE \"{table}\" ADD COLUMN \"{column}\" {def}");
    }

    static void Exec(SqliteConnection conn, string sql)
    {
        using var cmd = conn.CreateCommand();
        cmd.CommandText = sql;
        cmd.ExecuteNonQuery();
    }

    // CREATE TABLE IF NOT EXISTS for every table InitialSchema creates.
    // Indices use IF NOT EXISTS so re-runs are safe.
    static readonly string[] InitialSchemaSql =
    [
        """
        CREATE TABLE IF NOT EXISTS "server_settings" (
            "key" TEXT NOT NULL CONSTRAINT "PK_server_settings" PRIMARY KEY,
            "value" TEXT NOT NULL
        )
        """,
        """
        CREATE TABLE IF NOT EXISTS "title_meta" (
            "title_key" TEXT NOT NULL CONSTRAINT "PK_title_meta" PRIMARY KEY,
            "cover_local_path" TEXT,
            "cover_cdn_url" TEXT,
            "description" TEXT,
            "tags" TEXT,
            "chapter_count" INTEGER NOT NULL,
            "source" TEXT NOT NULL,
            "source_id" TEXT NOT NULL,
            "fetched_at" TEXT NOT NULL
        )
        """,
        """
        CREATE TABLE IF NOT EXISTS "titles" (
            "id" TEXT NOT NULL CONSTRAINT "PK_titles" PRIMARY KEY,
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
            "local_path" TEXT
        )
        """,
        "CREATE UNIQUE INDEX IF NOT EXISTS \"IX_titles_mangaupdates_id\" ON \"titles\" (\"mangaupdates_id\") WHERE mangaupdates_id IS NOT NULL",
        """
        CREATE TABLE IF NOT EXISTS "users" (
            "id" TEXT NOT NULL CONSTRAINT "PK_users" PRIMARY KEY,
            "username" TEXT NOT NULL,
            "password_hash" TEXT NOT NULL,
            "created_at" TEXT NOT NULL,
            "role" TEXT NOT NULL,
            "allow_explicit" INTEGER NOT NULL DEFAULT 0
        )
        """,
        "CREATE UNIQUE INDEX IF NOT EXISTS \"IX_users_username\" ON \"users\" (\"username\")",
        """
        CREATE TABLE IF NOT EXISTS "chapters" (
            "id" TEXT NOT NULL CONSTRAINT "PK_chapters" PRIMARY KEY,
            "title_id" TEXT NOT NULL,
            "title" TEXT,
            "number" REAL NOT NULL,
            "volume" REAL,
            "local_path" TEXT,
            "page_count" INTEGER NOT NULL,
            "downloaded" INTEGER NOT NULL,
            "created_at" TEXT NOT NULL,
            "is_new" INTEGER NOT NULL,
            "chapter_format" TEXT NOT NULL,
            CONSTRAINT "FK_chapters_titles_title_id" FOREIGN KEY ("title_id") REFERENCES "titles" ("id") ON DELETE CASCADE
        )
        """,
        "CREATE INDEX IF NOT EXISTS \"idx_chapters_title_id\" ON \"chapters\" (\"title_id\")",
        """
        CREATE TABLE IF NOT EXISTS "chapter_sources" (
            "id" TEXT NOT NULL CONSTRAINT "PK_chapter_sources" PRIMARY KEY,
            "chapter_id" TEXT NOT NULL,
            "source" TEXT NOT NULL,
            "source_id" TEXT NOT NULL,
            CONSTRAINT "FK_chapter_sources_chapters_chapter_id" FOREIGN KEY ("chapter_id") REFERENCES "chapters" ("id") ON DELETE CASCADE
        )
        """,
        "CREATE INDEX IF NOT EXISTS \"idx_chapter_sources_chapter_id\" ON \"chapter_sources\" (\"chapter_id\")",
        "CREATE UNIQUE INDEX IF NOT EXISTS \"IX_chapter_sources_chapter_id_source\" ON \"chapter_sources\" (\"chapter_id\", \"source\")",
        """
        CREATE TABLE IF NOT EXISTS "download_queue" (
            "id" TEXT NOT NULL CONSTRAINT "PK_download_queue" PRIMARY KEY,
            "chapter_id" TEXT NOT NULL,
            "manga_title" TEXT NOT NULL,
            "chapter_num" REAL NOT NULL,
            "status" TEXT NOT NULL,
            "error" TEXT,
            "created_at" TEXT NOT NULL,
            "updated_at" TEXT NOT NULL,
            "pages_downloaded" INTEGER NOT NULL,
            "pages_total" INTEGER NOT NULL,
            "queued_by" TEXT,
            CONSTRAINT "FK_download_queue_chapters_chapter_id" FOREIGN KEY ("chapter_id") REFERENCES "chapters" ("id") ON DELETE CASCADE,
            CONSTRAINT "FK_download_queue_users_queued_by" FOREIGN KEY ("queued_by") REFERENCES "users" ("id") ON DELETE SET NULL
        )
        """,
        "CREATE INDEX IF NOT EXISTS \"idx_queue_status\" ON \"download_queue\" (\"status\", \"created_at\")",
        "CREATE UNIQUE INDEX IF NOT EXISTS \"IX_download_queue_chapter_id\" ON \"download_queue\" (\"chapter_id\")",
        "CREATE INDEX IF NOT EXISTS \"IX_download_queue_queued_by\" ON \"download_queue\" (\"queued_by\")",
        """
        CREATE TABLE IF NOT EXISTS "read_progress" (
            "id" TEXT NOT NULL CONSTRAINT "PK_read_progress" PRIMARY KEY,
            "user_id" TEXT NOT NULL,
            "chapter_id" TEXT NOT NULL,
            "current_page" INTEGER NOT NULL,
            "completed" INTEGER NOT NULL,
            "updated_at" TEXT NOT NULL,
            CONSTRAINT "FK_read_progress_chapters_chapter_id" FOREIGN KEY ("chapter_id") REFERENCES "chapters" ("id") ON DELETE CASCADE,
            CONSTRAINT "FK_read_progress_users_user_id" FOREIGN KEY ("user_id") REFERENCES "users" ("id") ON DELETE CASCADE
        )
        """,
        "CREATE INDEX IF NOT EXISTS \"idx_read_progress_chapter\" ON \"read_progress\" (\"chapter_id\")",
        "CREATE INDEX IF NOT EXISTS \"idx_read_progress_user\" ON \"read_progress\" (\"user_id\")",
        "CREATE UNIQUE INDEX IF NOT EXISTS \"IX_read_progress_user_id_chapter_id\" ON \"read_progress\" (\"user_id\", \"chapter_id\")",
        """
        CREATE TABLE IF NOT EXISTS "sync_log" (
            "id" TEXT NOT NULL CONSTRAINT "PK_sync_log" PRIMARY KEY,
            "title_id" TEXT NOT NULL,
            "message" TEXT NOT NULL,
            "created_at" TEXT NOT NULL,
            CONSTRAINT "FK_sync_log_titles_title_id" FOREIGN KEY ("title_id") REFERENCES "titles" ("id") ON DELETE CASCADE
        )
        """,
        "CREATE INDEX IF NOT EXISTS \"idx_sync_log_title_id\" ON \"sync_log\" (\"title_id\", \"created_at\")",
        """
        CREATE TABLE IF NOT EXISTS "sync_warnings" (
            "id" TEXT NOT NULL CONSTRAINT "PK_sync_warnings" PRIMARY KEY,
            "title_id" TEXT NOT NULL,
            "plugin_id" TEXT NOT NULL,
            "message" TEXT NOT NULL,
            "created_at" TEXT NOT NULL,
            CONSTRAINT "FK_sync_warnings_titles_title_id" FOREIGN KEY ("title_id") REFERENCES "titles" ("id") ON DELETE CASCADE
        )
        """,
        "CREATE INDEX IF NOT EXISTS \"idx_sync_warnings_title_id\" ON \"sync_warnings\" (\"title_id\")",
        "CREATE UNIQUE INDEX IF NOT EXISTS \"IX_sync_warnings_title_id_plugin_id\" ON \"sync_warnings\" (\"title_id\", \"plugin_id\")",
        """
        CREATE TABLE IF NOT EXISTS "title_aliases" (
            "id" TEXT NOT NULL CONSTRAINT "PK_title_aliases" PRIMARY KEY,
            "title_id" TEXT NOT NULL,
            "alias" TEXT NOT NULL,
            CONSTRAINT "FK_title_aliases_titles_title_id" FOREIGN KEY ("title_id") REFERENCES "titles" ("id") ON DELETE CASCADE
        )
        """,
        "CREATE INDEX IF NOT EXISTS \"idx_title_aliases_title_id\" ON \"title_aliases\" (\"title_id\")",
        """
        CREATE TABLE IF NOT EXISTS "title_sources" (
            "id" TEXT NOT NULL CONSTRAINT "PK_title_sources" PRIMARY KEY,
            "title_id" TEXT NOT NULL,
            "source" TEXT NOT NULL,
            "source_id" TEXT NOT NULL,
            "discovered_at" TEXT NOT NULL,
            CONSTRAINT "FK_title_sources_titles_title_id" FOREIGN KEY ("title_id") REFERENCES "titles" ("id") ON DELETE CASCADE
        )
        """,
        "CREATE UNIQUE INDEX IF NOT EXISTS \"IX_title_sources_title_id_source\" ON \"title_sources\" (\"title_id\", \"source\")",
        """
        CREATE TABLE IF NOT EXISTS "user_title_settings" (
            "user_id" TEXT NOT NULL,
            "title_id" TEXT NOT NULL,
            "reader_mode" TEXT,
            CONSTRAINT "PK_user_title_settings" PRIMARY KEY ("user_id", "title_id"),
            CONSTRAINT "FK_user_title_settings_titles_title_id" FOREIGN KEY ("title_id") REFERENCES "titles" ("id") ON DELETE CASCADE,
            CONSTRAINT "FK_user_title_settings_users_user_id" FOREIGN KEY ("user_id") REFERENCES "users" ("id") ON DELETE CASCADE
        )
        """,
        "CREATE INDEX IF NOT EXISTS \"IX_user_title_settings_title_id\" ON \"user_title_settings\" (\"title_id\")",
        """
        CREATE TABLE IF NOT EXISTS "user_titles" (
            "user_id" TEXT NOT NULL,
            "title_id" TEXT NOT NULL,
            "added_at" TEXT NOT NULL,
            CONSTRAINT "PK_user_titles" PRIMARY KEY ("user_id", "title_id"),
            CONSTRAINT "FK_user_titles_titles_title_id" FOREIGN KEY ("title_id") REFERENCES "titles" ("id") ON DELETE CASCADE,
            CONSTRAINT "FK_user_titles_users_user_id" FOREIGN KEY ("user_id") REFERENCES "users" ("id") ON DELETE CASCADE
        )
        """,
        "CREATE INDEX IF NOT EXISTS \"idx_user_titles_user\" ON \"user_titles\" (\"user_id\")",
        "CREATE INDEX IF NOT EXISTS \"IX_user_titles_title_id\" ON \"user_titles\" (\"title_id\")",
    ];
}
