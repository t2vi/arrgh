//! `read_progress` access + continue-reading query (ADR 0033, S4 #126). Port
//! of the data half of `Api/Progress.cs`. No sqlx migrations — same
//! deferral as `titles.rs`.

use sqlx::{FromRow, SqlitePool};
use time::macros::format_description;
use time::OffsetDateTime;

fn ef_timestamp_now() -> String {
    let fmt =
        format_description!("[year]-[month]-[day] [hour]:[minute]:[second].[subsecond digits:6]");
    OffsetDateTime::now_utc()
        .format(&fmt)
        .expect("format is a static valid pattern")
}

#[derive(FromRow)]
pub struct ReadProgressRow {
    pub id: String,
    pub user_id: String,
    pub chapter_id: String,
    pub current_page: i64,
    pub completed: bool,
    pub updated_at: String,
}

pub async fn list_title_progress(
    pool: &SqlitePool,
    user_id: &str,
    title_id: &str,
) -> sqlx::Result<Vec<ReadProgressRow>> {
    sqlx::query_as(
        "SELECT rp.id, rp.user_id, rp.chapter_id, rp.current_page, rp.completed, rp.updated_at \
         FROM read_progress rp JOIN chapters c ON c.id = rp.chapter_id \
         WHERE rp.user_id = ? AND c.title_id = ?",
    )
    .bind(user_id)
    .bind(title_id)
    .fetch_all(pool)
    .await
}

pub async fn get_progress(
    pool: &SqlitePool,
    chapter_id: &str,
    user_id: &str,
) -> sqlx::Result<Option<ReadProgressRow>> {
    sqlx::query_as(
        "SELECT id, user_id, chapter_id, current_page, completed, updated_at \
         FROM read_progress WHERE chapter_id = ? AND user_id = ?",
    )
    .bind(chapter_id)
    .bind(user_id)
    .fetch_optional(pool)
    .await
}

/// Upsert on the `(user_id, chapter_id)` unique index — mirrors
/// `Progress.cs`'s raw-SQL `ON CONFLICT` upsert (no EF equivalent there
/// either; same reasoning applies here).
pub async fn upsert_progress(
    pool: &SqlitePool,
    user_id: &str,
    chapter_id: &str,
    current_page: i64,
    completed: bool,
) -> sqlx::Result<ReadProgressRow> {
    let id = uuid::Uuid::new_v4().to_string();
    let now = ef_timestamp_now();
    sqlx::query(
        "INSERT INTO read_progress (id, user_id, chapter_id, current_page, completed, updated_at) \
         VALUES (?, ?, ?, ?, ?, ?) \
         ON CONFLICT(user_id, chapter_id) DO UPDATE SET \
             current_page = excluded.current_page, \
             completed    = excluded.completed, \
             updated_at   = excluded.updated_at",
    )
    .bind(&id)
    .bind(user_id)
    .bind(chapter_id)
    .bind(current_page)
    .bind(completed)
    .bind(&now)
    .execute(pool)
    .await?;

    get_progress(pool, chapter_id, user_id)
        .await?
        .ok_or(sqlx::Error::RowNotFound)
}

#[derive(FromRow)]
pub struct ContinueItem {
    pub title_id: String,
    pub manga_title: String,
    pub cover_url: Option<String>,
    pub chapter_id: Option<String>,
    pub chapter_number: Option<f64>,
    pub chapters_read: i64,
    pub total_chapters: i64,
}

pub async fn continue_reading(pool: &SqlitePool, user_id: &str) -> sqlx::Result<Vec<ContinueItem>> {
    sqlx::query_as(
        "SELECT t.id AS title_id, t.title AS manga_title, t.cover_url AS cover_url, \
         (SELECT c.id FROM chapters c WHERE c.title_id = t.id AND c.downloaded = 1 \
            AND NOT EXISTS(SELECT 1 FROM read_progress rp WHERE rp.chapter_id = c.id AND rp.user_id = ? AND rp.completed = 1) \
            ORDER BY c.number LIMIT 1) AS chapter_id, \
         (SELECT c.number FROM chapters c WHERE c.title_id = t.id AND c.downloaded = 1 \
            AND NOT EXISTS(SELECT 1 FROM read_progress rp WHERE rp.chapter_id = c.id AND rp.user_id = ? AND rp.completed = 1) \
            ORDER BY c.number LIMIT 1) AS chapter_number, \
         (SELECT COUNT(*) FROM read_progress rp2 JOIN chapters c2 ON c2.id = rp2.chapter_id \
            WHERE rp2.user_id = ? AND rp2.completed = 1 AND c2.title_id = t.id) AS chapters_read, \
         (SELECT COUNT(*) FROM chapters c3 WHERE c3.title_id = t.id) AS total_chapters \
         FROM titles t \
         WHERE EXISTS(SELECT 1 FROM read_progress rp3 JOIN chapters c4 ON c4.id = rp3.chapter_id \
            WHERE rp3.user_id = ? AND rp3.completed = 1 AND c4.title_id = t.id) \
         AND EXISTS(SELECT 1 FROM chapters c5 WHERE c5.title_id = t.id AND c5.downloaded = 1 \
            AND NOT EXISTS(SELECT 1 FROM read_progress rp4 WHERE rp4.chapter_id = c5.id AND rp4.user_id = ? AND rp4.completed = 1)) \
         ORDER BY t.updated_at DESC LIMIT 10",
    )
    .bind(user_id)
    .bind(user_id)
    .bind(user_id)
    .bind(user_id)
    .bind(user_id)
    .fetch_all(pool)
    .await
}
