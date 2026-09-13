//! Shared integration-test helpers. Each test file that `mod common;`s this
//! gets its own isolated temp-file SQLite DB per `build_state()` call.
//!
//! `mod common;` is recompiled per test binary (version.rs, logs.rs,
//! auth.rs, ...) — a helper only some of them use isn't dead code overall.
#![allow(dead_code)]

use arrgh_server::config::Config;
use arrgh_server::logs::LogBuffer;
use arrgh_server::state::{connect_db, AppState};

pub const JWT_SECRET: &str = "integration-test-secret-32chars!";

/// `users` table schema as EF actually creates it (verified against a live
/// `.NET`-migrated DB) — see `src/users.rs`'s module doc for the exact
/// column/format contract this mirrors.
const USERS_SCHEMA: &str = r#"
CREATE TABLE IF NOT EXISTS users (
    id TEXT NOT NULL PRIMARY KEY,
    username TEXT NOT NULL,
    password_hash TEXT NOT NULL,
    created_at TEXT NOT NULL,
    role TEXT NOT NULL,
    allow_explicit INTEGER NOT NULL
);
CREATE UNIQUE INDEX IF NOT EXISTS IX_users_username ON users (username);

CREATE TABLE IF NOT EXISTS server_settings (
    key TEXT NOT NULL PRIMARY KEY,
    value TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS external_sources (
    id TEXT NOT NULL PRIMARY KEY,
    name TEXT NOT NULL,
    base_url TEXT NOT NULL,
    api_key TEXT,
    content_types TEXT NOT NULL,
    enabled INTEGER NOT NULL,
    created_at TEXT NOT NULL,
    is_community INTEGER NOT NULL,
    priority INTEGER NOT NULL,
    source_key TEXT,
    default_explicit INTEGER NOT NULL
);

-- S4 (#126) — titles/progress schema, verified against
-- Migrations/20260529064158_InitialSchema.cs.
CREATE TABLE IF NOT EXISTS titles (
    id TEXT NOT NULL PRIMARY KEY,
    title TEXT NOT NULL,
    description TEXT,
    cover_url TEXT,
    status TEXT NOT NULL,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    author TEXT,
    year INTEGER,
    tags TEXT,
    sync_status TEXT NOT NULL,
    content_type TEXT NOT NULL,
    auto_download INTEGER,
    reader_mode TEXT,
    download_dir TEXT,
    is_explicit INTEGER NOT NULL,
    mangaupdates_id TEXT,
    local_path TEXT
);

CREATE TABLE IF NOT EXISTS chapters (
    id TEXT NOT NULL PRIMARY KEY,
    title_id TEXT NOT NULL REFERENCES titles(id) ON DELETE CASCADE,
    title TEXT,
    number REAL NOT NULL,
    volume REAL,
    local_path TEXT,
    page_count INTEGER NOT NULL,
    downloaded INTEGER NOT NULL,
    created_at TEXT NOT NULL,
    is_new INTEGER NOT NULL,
    chapter_format TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS sync_log (
    id TEXT NOT NULL PRIMARY KEY,
    title_id TEXT NOT NULL REFERENCES titles(id) ON DELETE CASCADE,
    message TEXT NOT NULL,
    created_at TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS sync_warnings (
    id TEXT NOT NULL PRIMARY KEY,
    title_id TEXT NOT NULL REFERENCES titles(id) ON DELETE CASCADE,
    plugin_id TEXT NOT NULL,
    message TEXT NOT NULL,
    created_at TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS title_aliases (
    id TEXT NOT NULL PRIMARY KEY,
    title_id TEXT NOT NULL REFERENCES titles(id) ON DELETE CASCADE,
    alias TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS title_sources (
    id TEXT NOT NULL PRIMARY KEY,
    title_id TEXT NOT NULL REFERENCES titles(id) ON DELETE CASCADE,
    source TEXT NOT NULL,
    source_id TEXT NOT NULL,
    discovered_at TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS user_title_settings (
    user_id TEXT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    title_id TEXT NOT NULL REFERENCES titles(id) ON DELETE CASCADE,
    reader_mode TEXT,
    PRIMARY KEY (user_id, title_id)
);

CREATE TABLE IF NOT EXISTS user_titles (
    user_id TEXT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    title_id TEXT NOT NULL REFERENCES titles(id) ON DELETE CASCADE,
    added_at TEXT NOT NULL,
    PRIMARY KEY (user_id, title_id)
);

CREATE TABLE IF NOT EXISTS read_progress (
    id TEXT NOT NULL PRIMARY KEY,
    user_id TEXT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    chapter_id TEXT NOT NULL REFERENCES chapters(id) ON DELETE CASCADE,
    current_page INTEGER NOT NULL,
    completed INTEGER NOT NULL,
    updated_at TEXT NOT NULL
);
CREATE UNIQUE INDEX IF NOT EXISTS IX_read_progress_user_id_chapter_id ON read_progress (user_id, chapter_id);
"#;

pub async fn build_state() -> AppState {
    let db_path = std::env::temp_dir().join(format!("arrgh-rust-test-{}.db", uuid::Uuid::new_v4()));
    let db_path = db_path.to_str().unwrap().to_string();

    let config = Config {
        jwt_secret: Some(JWT_SECRET.into()),
        database_path: db_path.clone(),
        ..Config::from_env().expect("default config")
    };

    let db = connect_db(&db_path).await.expect("connect test db");
    for stmt in USERS_SCHEMA
        .split(';')
        .map(str::trim)
        .filter(|s| !s.is_empty())
    {
        sqlx::query(stmt)
            .execute(&db)
            .await
            .expect("create test schema");
    }

    AppState::new(config, LogBuffer::new("info"), db)
}

/// Inserts a user directly (bypassing the API) for tests that need one to
/// already exist. Fixed password `"password123"` — mirrors the .NET
/// `Fake.AdminUser`/`Fake.MemberUser` fixtures.
pub async fn seed_user(
    state: &AppState,
    username: &str,
    role: &str,
    allow_explicit: bool,
) -> arrgh_server::users::UserRow {
    let id = uuid::Uuid::new_v4().to_string();
    let hash = arrgh_server::auth::hash_password("password123").unwrap();
    arrgh_server::users::insert(&state.db, &id, username, &hash, role, allow_explicit)
        .await
        .unwrap();
    arrgh_server::users::find_by_id(&state.db, &id)
        .await
        .unwrap()
        .unwrap()
}

pub fn token_for(user: &arrgh_server::users::UserRow) -> String {
    arrgh_server::auth::create_token(
        &user.id,
        &user.username,
        &user.role,
        user.allow_explicit,
        JWT_SECRET,
    )
    .unwrap()
}

/// Timestamp for seeded rows — monotonic enough across sequential awaited
/// inserts within one test for ORDER BY created_at to be deterministic.
fn now_str() -> String {
    use time::macros::format_description;
    time::OffsetDateTime::now_utc()
        .format(&format_description!(
            "[year]-[month]-[day] [hour]:[minute]:[second].[subsecond digits:6]"
        ))
        .unwrap()
}

/// Inserts a title row directly, returning its id. Defaults: status
/// "ongoing", sync_status "ready", content_type "manga", not explicit.
pub async fn seed_title(state: &AppState, title: &str, is_explicit: bool) -> String {
    let id = uuid::Uuid::new_v4().to_string();
    let now = now_str();
    sqlx::query(
        "INSERT INTO titles (id, title, status, created_at, updated_at, sync_status, content_type, is_explicit) \
         VALUES (?, ?, 'ongoing', ?, ?, 'ready', 'manga', ?)",
    )
    .bind(&id)
    .bind(title)
    .bind(&now)
    .bind(&now)
    .bind(is_explicit)
    .execute(&state.db)
    .await
    .unwrap();
    id
}

/// Adds `user_id` as an owner of `title_id` (`user_titles` row).
pub async fn seed_user_title(state: &AppState, user_id: &str, title_id: &str) {
    sqlx::query("INSERT INTO user_titles (user_id, title_id, added_at) VALUES (?, ?, ?)")
        .bind(user_id)
        .bind(title_id)
        .bind(now_str())
        .execute(&state.db)
        .await
        .unwrap();
}

/// Inserts a chapter row, returning its id. `downloaded` defaults false.
pub async fn seed_chapter(
    state: &AppState,
    title_id: &str,
    number: f64,
    downloaded: bool,
) -> String {
    let id = uuid::Uuid::new_v4().to_string();
    sqlx::query(
        "INSERT INTO chapters (id, title_id, number, page_count, downloaded, created_at, is_new, chapter_format) \
         VALUES (?, ?, ?, 0, ?, ?, 0, 'cbz')",
    )
    .bind(&id)
    .bind(title_id)
    .bind(number)
    .bind(downloaded)
    .bind(now_str())
    .execute(&state.db)
    .await
    .unwrap();
    id
}

pub async fn mark_new(state: &AppState, chapter_id: &str) {
    sqlx::query("UPDATE chapters SET is_new = 1 WHERE id = ?")
        .bind(chapter_id)
        .execute(&state.db)
        .await
        .unwrap();
}

pub async fn mark_read(state: &AppState, user_id: &str, chapter_id: &str) {
    sqlx::query(
        "INSERT INTO read_progress (id, user_id, chapter_id, current_page, completed, updated_at) \
         VALUES (?, ?, ?, 0, 1, ?)",
    )
    .bind(uuid::Uuid::new_v4().to_string())
    .bind(user_id)
    .bind(chapter_id)
    .bind(now_str())
    .execute(&state.db)
    .await
    .unwrap();
}

pub async fn add_title_source(state: &AppState, title_id: &str, source: &str) {
    sqlx::query(
        "INSERT INTO title_sources (id, title_id, source, source_id, discovered_at) VALUES (?, ?, ?, 'src-1', ?)",
    )
    .bind(uuid::Uuid::new_v4().to_string())
    .bind(title_id)
    .bind(source)
    .bind(now_str())
    .execute(&state.db)
    .await
    .unwrap();
}

pub async fn add_sync_warning(state: &AppState, title_id: &str) {
    sqlx::query(
        "INSERT INTO sync_warnings (id, title_id, plugin_id, message, created_at) VALUES (?, ?, 'plugin-1', 'warn', ?)",
    )
    .bind(uuid::Uuid::new_v4().to_string())
    .bind(title_id)
    .bind(now_str())
    .execute(&state.db)
    .await
    .unwrap();
}

pub async fn add_sync_log(state: &AppState, title_id: &str, message: &str) {
    sqlx::query("INSERT INTO sync_log (id, title_id, message, created_at) VALUES (?, ?, ?, ?)")
        .bind(uuid::Uuid::new_v4().to_string())
        .bind(title_id)
        .bind(message)
        .bind(now_str())
        .execute(&state.db)
        .await
        .unwrap();
}

/// Inserts an external source row directly, returning its id.
pub async fn seed_source(
    state: &AppState,
    name: &str,
    base_url: &str,
    content_types: &str,
    api_key: Option<&str>,
) -> String {
    let id = uuid::Uuid::new_v4().to_string();
    sqlx::query(
        "INSERT INTO external_sources (id, name, base_url, api_key, content_types, enabled, created_at, is_community, priority, default_explicit) VALUES (?, ?, ?, ?, ?, 1, datetime('now'), 0, 100, 0)",
    )
    .bind(&id)
    .bind(name)
    .bind(base_url)
    .bind(api_key)
    .bind(content_types)
    .execute(&state.db)
    .await
    .unwrap();
    id
}
