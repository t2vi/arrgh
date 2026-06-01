using ArrghServer.Data;
using Microsoft.Data.Sqlite;
using Microsoft.EntityFrameworkCore;

namespace ArrghServer;

public static class MigrationBootstrap
{
    const string V = "10.0.8";

    // Called before db.Database.Migrate() to handle databases created before EF migrations
    // were introduced (e.g. from the original Rust server or a pre-migration .NET build).
    // If __EFMigrationsHistory is absent but tables exist, we create the history table and
    // record migrations as applied so Migrate() only runs genuinely new migrations.
    public static void Bootstrap(AppDbContext db)
    {
        db.Database.OpenConnection();
        try
        {
            var conn = (SqliteConnection)db.Database.GetDbConnection();

            using var check = conn.CreateCommand();
            check.CommandText = "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name='__EFMigrationsHistory'";
            if ((long)check.ExecuteScalar()! > 0) return;

            check.CommandText = "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name='external_sources'";
            if ((long)check.ExecuteScalar()! == 0) return;

            using var cmd = conn.CreateCommand();
            cmd.CommandText = $"""
                CREATE TABLE "__EFMigrationsHistory" (
                    "MigrationId" TEXT NOT NULL CONSTRAINT "PK___EFMigrationsHistory" PRIMARY KEY,
                    "ProductVersion" TEXT NOT NULL
                );
                INSERT INTO "__EFMigrationsHistory" VALUES ('20260529064158_InitialSchema', '{V}');
                """;
            cmd.ExecuteNonQuery();

            check.CommandText = "SELECT COUNT(*) FROM pragma_table_info('titles') WHERE name='metadata_source'";
            if ((long)check.ExecuteScalar()! > 0)
            {
                cmd.CommandText = $"INSERT INTO \"__EFMigrationsHistory\" VALUES ('20260529104924_AddMetadataSourceColumns', '{V}')";
                cmd.ExecuteNonQuery();
            }
        }
        finally
        {
            db.Database.CloseConnection();
        }
    }
}
