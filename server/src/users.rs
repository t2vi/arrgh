//! `users` table access (ADR 0033, S2 #124). No sqlx migrations yet — the
//! .NET server still owns schema/migrations for every table until cutover
//! (S10, #132); this just queries the `users` table EF already created.
//! Column names and the `created_at` text format (`yyyy-MM-dd
//! HH:mm:ss.ffffff`, verified against a live EF-written row) must match
//! exactly since both servers read/write the same SQLite file during the
//! strangler-fig.

use sqlx::{FromRow, SqlitePool};
use time::macros::format_description;
use time::OffsetDateTime;

#[derive(FromRow, Clone)]
pub struct UserRow {
    pub id: String,
    pub username: String,
    pub password_hash: String,
    pub created_at: String,
    pub role: String,
    pub allow_explicit: bool,
}

pub async fn count(pool: &SqlitePool) -> sqlx::Result<i64> {
    sqlx::query_scalar("SELECT COUNT(*) FROM users")
        .fetch_one(pool)
        .await
}

pub async fn exists_by_username(pool: &SqlitePool, username: &str) -> sqlx::Result<bool> {
    let n: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM users WHERE username = ?")
        .bind(username)
        .fetch_one(pool)
        .await?;
    Ok(n > 0)
}

pub async fn find_by_username(pool: &SqlitePool, username: &str) -> sqlx::Result<Option<UserRow>> {
    sqlx::query_as(
        "SELECT id, username, password_hash, created_at, role, allow_explicit FROM users WHERE username = ?",
    )
    .bind(username)
    .fetch_optional(pool)
    .await
}

pub async fn find_by_id(pool: &SqlitePool, id: &str) -> sqlx::Result<Option<UserRow>> {
    sqlx::query_as(
        "SELECT id, username, password_hash, created_at, role, allow_explicit FROM users WHERE id = ?",
    )
    .bind(id)
    .fetch_optional(pool)
    .await
}

pub async fn list_ordered_by_created_at(pool: &SqlitePool) -> sqlx::Result<Vec<UserRow>> {
    sqlx::query_as(
        "SELECT id, username, password_hash, created_at, role, allow_explicit FROM users ORDER BY created_at",
    )
    .fetch_all(pool)
    .await
}

pub async fn insert(
    pool: &SqlitePool,
    id: &str,
    username: &str,
    password_hash: &str,
    role: &str,
    allow_explicit: bool,
) -> sqlx::Result<()> {
    sqlx::query(
        "INSERT INTO users (id, username, password_hash, created_at, role, allow_explicit) VALUES (?, ?, ?, ?, ?, ?)",
    )
    .bind(id)
    .bind(username)
    .bind(password_hash)
    .bind(ef_timestamp_now())
    .bind(role)
    .bind(allow_explicit)
    .execute(pool)
    .await?;
    Ok(())
}

pub async fn update_password(pool: &SqlitePool, id: &str, password_hash: &str) -> sqlx::Result<()> {
    sqlx::query("UPDATE users SET password_hash = ? WHERE id = ?")
        .bind(password_hash)
        .bind(id)
        .execute(pool)
        .await?;
    Ok(())
}

pub async fn update_allow_explicit(
    pool: &SqlitePool,
    id: &str,
    allow_explicit: bool,
) -> sqlx::Result<()> {
    sqlx::query("UPDATE users SET allow_explicit = ? WHERE id = ?")
        .bind(allow_explicit)
        .bind(id)
        .execute(pool)
        .await?;
    Ok(())
}

pub async fn update_role(pool: &SqlitePool, id: &str, role: &str) -> sqlx::Result<()> {
    sqlx::query("UPDATE users SET role = ? WHERE id = ?")
        .bind(role)
        .bind(id)
        .execute(pool)
        .await?;
    Ok(())
}

/// `true` if a row was actually deleted (caller maps absence to 404).
pub async fn delete(pool: &SqlitePool, id: &str) -> sqlx::Result<bool> {
    let res = sqlx::query("DELETE FROM users WHERE id = ?")
        .bind(id)
        .execute(pool)
        .await?;
    Ok(res.rows_affected() > 0)
}

fn ef_timestamp_now() -> String {
    let fmt =
        format_description!("[year]-[month]-[day] [hour]:[minute]:[second].[subsecond digits:6]");
    OffsetDateTime::now_utc()
        .format(&fmt)
        .expect("format is a static valid pattern")
}
