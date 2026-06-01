using ArrghServer;
using ArrghServer.Data;
using ArrghServer.Data.Models;
using Microsoft.Data.Sqlite;
using Microsoft.EntityFrameworkCore;

namespace ArrghServer.Tests;

public class MigrationBootstrapTests
{
    [Fact]
    [Trait("Category", TestCategories.Unit)]
    public void Bootstrap_PreMigrationDb_CreatesHistoryAndMigrateSucceeds()
    {
        var dbPath = Path.Combine(Path.GetTempPath(), $"bootstrap-pre-{Guid.NewGuid():N}.db");
        try
        {
            var opts = new DbContextOptionsBuilder<AppDbContext>()
                .UseSqlite($"Data Source={dbPath}")
                .Options;

            // Simulate pre-migration DB: EnsureCreated creates all tables from the current model
            // snapshot but does NOT create __EFMigrationsHistory
            using (var db = new AppDbContext(opts))
                db.Database.EnsureCreated();

            // Pre-condition: no migration history table
            using (var conn = new SqliteConnection($"Data Source={dbPath}"))
            {
                conn.Open();
                using var cmd = conn.CreateCommand();
                cmd.CommandText = "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name='__EFMigrationsHistory'";
                Assert.Equal(0L, (long)cmd.ExecuteScalar()!);
            }

            // Bootstrap then Migrate must not throw
            using (var db = new AppDbContext(opts))
            {
                MigrationBootstrap.Bootstrap(db);
                db.Database.Migrate();
            }

            // Post-condition: __EFMigrationsHistory has both migration records
            using (var conn = new SqliteConnection($"Data Source={dbPath}"))
            {
                conn.Open();
                using var cmd = conn.CreateCommand();
                cmd.CommandText = "SELECT COUNT(*) FROM __EFMigrationsHistory";
                Assert.Equal(2L, (long)cmd.ExecuteScalar()!);
            }
        }
        finally
        {
            foreach (var f in new[] { dbPath, dbPath + "-shm", dbPath + "-wal" })
                if (File.Exists(f)) try { File.Delete(f); } catch { }
        }
    }

    [Fact]
    [Trait("Category", TestCategories.Unit)]
    public void Bootstrap_FreshDb_IsNoOpAndMigrateSucceeds()
    {
        var dbPath = Path.Combine(Path.GetTempPath(), $"bootstrap-fresh-{Guid.NewGuid():N}.db");
        try
        {
            var opts = new DbContextOptionsBuilder<AppDbContext>()
                .UseSqlite($"Data Source={dbPath}")
                .Options;

            using (var db = new AppDbContext(opts))
            {
                MigrationBootstrap.Bootstrap(db);

                // Fresh DB: Bootstrap should not have created __EFMigrationsHistory
                using var conn = new SqliteConnection($"Data Source={dbPath}");
                conn.Open();
                using var cmd = conn.CreateCommand();
                cmd.CommandText = "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name='__EFMigrationsHistory'";
                Assert.Equal(0L, (long)cmd.ExecuteScalar()!);
            }

            // Normal Migrate() handles fresh DB fine
            using (var db = new AppDbContext(opts))
                db.Database.Migrate();
        }
        finally
        {
            foreach (var f in new[] { dbPath, dbPath + "-shm", dbPath + "-wal" })
                if (File.Exists(f)) try { File.Delete(f); } catch { }
        }
    }

    [Fact]
    [Trait("Category", TestCategories.Unit)]
    public void Bootstrap_PartialSchemaDb_AddsNullableColumns_AndSaveChangesSucceeds()
    {
        // Simulate old Rust DB: external_sources exists but missing default_explicit, is_community, source_key
        var dbPath = Path.Combine(Path.GetTempPath(), $"bootstrap-partial-{Guid.NewGuid():N}.db");
        try
        {
            using (var conn = new SqliteConnection($"Data Source={dbPath}"))
            {
                conn.Open();
                using var cmd = conn.CreateCommand();
                cmd.CommandText = """
                    CREATE TABLE "external_sources" (
                        "id" TEXT NOT NULL PRIMARY KEY,
                        "name" TEXT NOT NULL,
                        "base_url" TEXT NOT NULL,
                        "api_key" TEXT,
                        "content_types" TEXT NOT NULL,
                        "enabled" INTEGER NOT NULL,
                        "created_at" TEXT NOT NULL,
                        "priority" INTEGER NOT NULL
                    );
                    INSERT INTO "external_sources" VALUES ('src1','MangaDex','http://plugin-host:4000',NULL,'manga',1,'2026-01-01',10);
                    """;
                cmd.ExecuteNonQuery();
            }

            var opts = new DbContextOptionsBuilder<AppDbContext>()
                .UseSqlite($"Data Source={dbPath}")
                .Options;

            // Bootstrap + Migrate must not throw
            using (var db = new AppDbContext(opts))
            {
                MigrationBootstrap.Bootstrap(db);
                db.Database.Migrate();
            }

            // SaveChanges must work — EF can UPDATE the row without hitting missing-column errors
            using (var db = new AppDbContext(opts))
            {
                var src = db.ExternalSources.Find("src1")!;
                src.Enabled = false;
                src.Priority = 99;
                db.SaveChanges();
            }

            using (var db = new AppDbContext(opts))
            {
                var src = db.ExternalSources.Find("src1")!;
                Assert.False(src.Enabled);
                Assert.Equal(99, src.Priority);
            }
        }
        finally
        {
            foreach (var f in new[] { dbPath, dbPath + "-shm", dbPath + "-wal" })
                if (File.Exists(f)) try { File.Delete(f); } catch { }
        }
    }

    [Fact]
    [Trait("Category", TestCategories.Unit)]
    public void Bootstrap_AlreadyMigratedDb_IsIdempotent()
    {
        var dbPath = Path.Combine(Path.GetTempPath(), $"bootstrap-migrated-{Guid.NewGuid():N}.db");
        try
        {
            var opts = new DbContextOptionsBuilder<AppDbContext>()
                .UseSqlite($"Data Source={dbPath}")
                .Options;

            // Normal first-run: Migrate creates schema + history
            using (var db = new AppDbContext(opts))
                db.Database.Migrate();

            // Bootstrap on already-migrated DB is a no-op; Migrate is still safe
            using (var db = new AppDbContext(opts))
            {
                MigrationBootstrap.Bootstrap(db);
                db.Database.Migrate();
            }
        }
        finally
        {
            foreach (var f in new[] { dbPath, dbPath + "-shm", dbPath + "-wal" })
                if (File.Exists(f)) try { File.Delete(f); } catch { }
        }
    }
}
