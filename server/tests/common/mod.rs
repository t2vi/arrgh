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
