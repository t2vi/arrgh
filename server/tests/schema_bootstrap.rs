//! S10 (#132) replacement for `server-tests/MigrationBootstrapTests.cs`.
//! The mechanism changed shape entirely (EF `__EFMigrationsHistory` class
//! migrations → `sqlx migrate` + idempotent baseline SQL — see
//! `src/state.rs`'s `connect_db` module doc) so these are new tests for
//! the new mechanism, not a line-for-line port.

use arrgh_server::state::connect_db;
use arrgh_server::titles;

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

    pool.close().await;
    std::fs::remove_file(&path).ok();
}

/// One link per (title, source) — a title can be linked to more than one source
/// (that's the whole point of the multi-source pool), but never twice to the
/// *same* source.
// spec: 002/FR-001
#[tokio::test]
async fn title_sources_rejects_a_duplicate_title_source_pair() {
    let path = temp_db_path("title-sources-unique");
    let pool = connect_db(&path).await.unwrap();
    let title_id = uuid::Uuid::new_v4().to_string();
    sqlx::query(
        "INSERT INTO titles (id, title, status, created_at, updated_at, sync_status, content_type, is_explicit) \
         VALUES (?, 'X', 'ongoing', datetime('now'), datetime('now'), 'ready', 'manga', 0)",
    )
    .bind(&title_id)
    .execute(&pool)
    .await
    .unwrap();

    titles::insert_title_source(&pool, &title_id, "mangadex", "src-1")
        .await
        .unwrap();
    // A second, different source for the same title is fine — that's FR-001's point.
    titles::insert_title_source(&pool, &title_id, "mangapill", "src-1")
        .await
        .unwrap();
    // The same source again is not.
    assert!(
        titles::insert_title_source(&pool, &title_id, "mangadex", "src-2")
            .await
            .is_err()
    );

    let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM title_sources WHERE title_id = ?")
        .bind(&title_id)
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(count, 2);

    pool.close().await;
    std::fs::remove_file(&path).ok();
}

/// One link per (chapter, source) — mirrors the title-level rule above at the
/// chapter level (spec 002 Key Entities: "Source Link (chapter-level)").
// spec: 002/FR-003
#[tokio::test]
async fn chapter_sources_rejects_a_duplicate_chapter_source_pair() {
    let path = temp_db_path("chapter-sources-unique");
    let pool = connect_db(&path).await.unwrap();
    let title_id = uuid::Uuid::new_v4().to_string();
    let chapter_id = uuid::Uuid::new_v4().to_string();
    sqlx::query(
        "INSERT INTO titles (id, title, status, created_at, updated_at, sync_status, content_type, is_explicit) \
         VALUES (?, 'X', 'ongoing', datetime('now'), datetime('now'), 'ready', 'manga', 0)",
    )
    .bind(&title_id)
    .execute(&pool)
    .await
    .unwrap();
    sqlx::query(
        "INSERT INTO chapters (id, title_id, number, chapter_format, is_new, page_count, downloaded, created_at) \
         VALUES (?, ?, 1.0, 'pages', 0, 0, 0, datetime('now'))",
    )
    .bind(&chapter_id)
    .bind(&title_id)
    .execute(&pool)
    .await
    .unwrap();

    let insert_link = |source: &'static str, source_id: &'static str| {
        let pool = pool.clone();
        let chapter_id = chapter_id.clone();
        async move {
            sqlx::query(
                "INSERT INTO chapter_sources (id, chapter_id, source, source_id) VALUES (?, ?, ?, ?)",
            )
            .bind(uuid::Uuid::new_v4().to_string())
            .bind(&chapter_id)
            .bind(source)
            .bind(source_id)
            .execute(&pool)
            .await
        }
    };

    insert_link("mangadex", "src-1").await.unwrap();
    // Two sources linked to the same chapter is exactly the multi-source pool's point.
    insert_link("mangapill", "src-1").await.unwrap();
    // The same source again is not.
    assert!(insert_link("mangadex", "src-2").await.is_err());

    let count: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM chapter_sources WHERE chapter_id = ?")
            .bind(&chapter_id)
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(count, 2);

    pool.close().await;
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

/// Spec 019 / ADR 0034: migration 0004 restores the Royal Road Source row
/// on existing installs (0003 deleted it), and no-ops on an empty table
/// (fresh installs get it from `DEFAULT_SOURCES`).
#[tokio::test]
async fn migration_0004_restores_royalroad_on_existing_install_only() {
    let path = temp_db_path("rr");

    // Fresh DB: 0004 runs against an empty external_sources → inserts nothing.
    let pool = connect_db(&path).await.unwrap();
    let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM external_sources")
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(count, 0);

    // Simulate an existing install that hasn't run 0004 yet.
    sqlx::query(
        "INSERT INTO external_sources (id, name, base_url, content_types, enabled, created_at, is_community, priority, source_key, default_explicit) \
         VALUES ('nf', 'NovelFull', 'http://ph:4000', 'novel', 1, datetime('now'), 0, 40, 'novelfull', 0)",
    )
    .execute(&pool)
    .await
    .unwrap();
    pool.close().await;
    for _ in 0..2 {
        // Twice: second pass proves idempotence.
        let p = connect_db(&path).await.unwrap();
        sqlx::query("DELETE FROM _sqlx_migrations WHERE version = 4")
            .execute(&p)
            .await
            .unwrap();
        p.close().await;
        let reopened = connect_db(&path).await.unwrap();
        let rows: Vec<(String, String, String, i64, i64, i64, i64)> = sqlx::query_as(
            "SELECT name, base_url, content_types, priority, enabled, is_community, default_explicit \
             FROM external_sources WHERE source_key = 'royalroad'",
        )
        .fetch_all(&reopened)
        .await
        .unwrap();
        assert_eq!(
            rows,
            vec![(
                "Royal Road".to_string(),
                "http://ph:4000".to_string(),
                "novel".to_string(),
                35,
                1,
                0,
                0
            )]
        );
        reopened.close().await;
    }

    std::fs::remove_file(&path).ok();
}

/// Spec 027: migration 0005 adds the NovelFull.net Source to existing
/// installs, and no-ops on an empty table (fresh installs get it from
/// `DEFAULT_SOURCES`).
#[tokio::test]
async fn migration_0005_adds_novelfullnet_on_existing_install_only() {
    let path = temp_db_path("nfn");

    let pool = connect_db(&path).await.unwrap();
    let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM external_sources")
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(count, 0);

    sqlx::query(
        "INSERT INTO external_sources (id, name, base_url, content_types, enabled, created_at, is_community, priority, source_key, default_explicit) \
         VALUES ('nf', 'NovelFull', 'http://ph:4000', 'novel', 1, datetime('now'), 0, 40, 'novelfull', 0)",
    )
    .execute(&pool)
    .await
    .unwrap();
    pool.close().await;
    for _ in 0..2 {
        // Twice: second pass proves idempotence.
        let p = connect_db(&path).await.unwrap();
        sqlx::query("DELETE FROM _sqlx_migrations WHERE version = 5")
            .execute(&p)
            .await
            .unwrap();
        p.close().await;
        let reopened = connect_db(&path).await.unwrap();
        let rows: Vec<(String, String, String, i64, i64, i64, i64)> = sqlx::query_as(
            "SELECT name, base_url, content_types, priority, enabled, is_community, default_explicit \
             FROM external_sources WHERE source_key = 'novelfullnet'",
        )
        .fetch_all(&reopened)
        .await
        .unwrap();
        assert_eq!(
            rows,
            vec![(
                "NovelFull.net".to_string(),
                "http://ph:4000".to_string(),
                "novel".to_string(),
                45,
                1,
                0,
                0
            )]
        );
        reopened.close().await;
    }

    std::fs::remove_file(&path).ok();
}
