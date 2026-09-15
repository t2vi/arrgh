//! S10 (#132) replacement for `server-tests/MigrationBootstrapTests.cs`.
//! The mechanism changed shape entirely (EF `__EFMigrationsHistory` class
//! migrations → `sqlx migrate` + idempotent baseline SQL — see
//! `src/state.rs`'s `connect_db` module doc) so these are new tests for
//! the new mechanism, not a line-for-line port.

use arrgh_server::state::connect_db;

fn temp_db_path(tag: &str) -> String {
    std::env::temp_dir()
        .join(format!("arrgh-bootstrap-{tag}-{}.db", uuid::Uuid::new_v4()))
        .to_str()
        .unwrap()
        .to_string()
}

#[tokio::test]
async fn fresh_db_gets_full_schema() {
    let path = temp_db_path("fresh");
    let pool = connect_db(&path).await.unwrap();

    let tables: Vec<String> = sqlx::query_scalar(
        "SELECT name FROM sqlite_master WHERE type='table' AND name NOT LIKE 'sqlite_%' AND name != '_sqlx_migrations'",
    )
    .fetch_all(&pool)
    .await
    .unwrap();

    for expected in [
        "external_sources",
        "server_settings",
        "title_meta",
        "titles",
        "users",
        "chapters",
        "sync_log",
        "sync_warnings",
        "title_aliases",
        "title_sources",
        "user_title_settings",
        "user_titles",
        "chapter_sources",
        "download_queue",
        "read_progress",
    ] {
        assert!(
            tables.iter().any(|t| t == expected),
            "missing table: {expected}"
        );
    }

    let has_metadata_source: bool = sqlx::query_scalar(
        "SELECT EXISTS(SELECT 1 FROM pragma_table_info('titles') WHERE name = 'metadata_source')",
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    assert!(has_metadata_source);

    std::fs::remove_file(&path).ok();
}

#[tokio::test]
async fn legacy_db_missing_metadata_columns_gets_upgraded() {
    let path = temp_db_path("legacy");

    // Simulate a DB frozen before EF's AddMetadataSourceColumns migration —
    // titles exists, but without metadata_source/metadata_source_id.
    {
        let opts = sqlx::sqlite::SqliteConnectOptions::new()
            .filename(&path)
            .create_if_missing(true);
        let pool = sqlx::SqlitePool::connect_with(opts).await.unwrap();
        sqlx::query(
            "CREATE TABLE titles (\
                id TEXT NOT NULL PRIMARY KEY, title TEXT NOT NULL, description TEXT, \
                cover_url TEXT, status TEXT NOT NULL, created_at TEXT NOT NULL, \
                updated_at TEXT NOT NULL, author TEXT, year INTEGER, tags TEXT, \
                sync_status TEXT NOT NULL, content_type TEXT NOT NULL, \
                auto_download INTEGER, reader_mode TEXT, download_dir TEXT, \
                is_explicit INTEGER NOT NULL, mangaupdates_id TEXT, local_path TEXT\
            )",
        )
        .execute(&pool)
        .await
        .unwrap();
        sqlx::query(
            "INSERT INTO titles (id, title, status, created_at, updated_at, sync_status, content_type, is_explicit) \
             VALUES ('t1', 'Naruto', 'ongoing', '2026-01-01', '2026-01-01', 'ready', 'manga', 0)",
        )
        .execute(&pool)
        .await
        .unwrap();
        pool.close().await;
    }

    // connect_db must patch the missing columns before migrate runs its
    // (no-op-against-an-existing-table) CREATE TABLE IF NOT EXISTS.
    let pool = connect_db(&path).await.unwrap();

    let has_columns: (bool, bool) = (
        sqlx::query_scalar(
            "SELECT EXISTS(SELECT 1 FROM pragma_table_info('titles') WHERE name = 'metadata_source')",
        )
        .fetch_one(&pool)
        .await
        .unwrap(),
        sqlx::query_scalar(
            "SELECT EXISTS(SELECT 1 FROM pragma_table_info('titles') WHERE name = 'metadata_source_id')",
        )
        .fetch_one(&pool)
        .await
        .unwrap(),
    );
    assert_eq!(has_columns, (true, true));

    // Pre-existing row survives, and can be updated through the new columns.
    sqlx::query("UPDATE titles SET metadata_source = 'mangaupdates' WHERE id = 't1'")
        .execute(&pool)
        .await
        .unwrap();
    let title: String = sqlx::query_scalar("SELECT title FROM titles WHERE id = 't1'")
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(title, "Naruto");

    std::fs::remove_file(&path).ok();
}

#[tokio::test]
async fn reconnecting_to_an_already_migrated_db_is_idempotent() {
    let path = temp_db_path("idempotent");

    let pool1 = connect_db(&path).await.unwrap();
    sqlx::query(
        "INSERT INTO users (id, username, password_hash, created_at, role, allow_explicit) \
         VALUES ('u1', 'admin', 'hash', '2026-01-01', 'admin', 0)",
    )
    .execute(&pool1)
    .await
    .unwrap();
    pool1.close().await;

    // Reconnect (simulates a restart) — must not error or touch existing data.
    let pool2 = connect_db(&path).await.unwrap();
    let username: String = sqlx::query_scalar("SELECT username FROM users WHERE id = 'u1'")
        .fetch_one(&pool2)
        .await
        .unwrap();
    assert_eq!(username, "admin");

    std::fs::remove_file(&path).ok();
}
