using ArrghServer;
using ArrghServer.Data;
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
