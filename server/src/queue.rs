//! `download_queue` access — port of `Api/Queue.cs`'s data half (ADR 0033,
//! S7 #129). `queue_download`'s insert lives in `chapters.rs` (S5 #127)
//! since the enqueue has no dependency on this module.

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
pub struct QueueItemRow {
    pub id: String,
    pub chapter_id: String,
    pub manga_title: String,
    pub chapter_num: f64,
    pub status: String,
    pub error: Option<String>,
    pub pages_downloaded: i64,
    pub pages_total: i64,
    pub created_at: String,
    pub updated_at: String,
}

const QUEUE_SELECT: &str = "SELECT \
    q.id, q.chapter_id, q.manga_title, q.chapter_num, q.status, q.error, \
    q.pages_downloaded, q.pages_total, q.created_at, q.updated_at \
    FROM download_queue q \
    JOIN chapters c ON c.id = q.chapter_id \
    JOIN titles t ON t.id = c.title_id";

pub async fn list(pool: &SqlitePool, allow_explicit: bool) -> sqlx::Result<Vec<QueueItemRow>> {
    sqlx::query_as(&format!(
        "{QUEUE_SELECT} WHERE (t.is_explicit = 0 OR ? = 1) ORDER BY q.created_at DESC LIMIT 100"
    ))
    .bind(allow_explicit as i64)
    .fetch_all(pool)
    .await
}

pub async fn list_for_title(
    pool: &SqlitePool,
    title_id: &str,
    allow_explicit: bool,
) -> sqlx::Result<Vec<QueueItemRow>> {
    sqlx::query_as(&format!(
        "{QUEUE_SELECT} WHERE c.title_id = ? AND (t.is_explicit = 0 OR ? = 1) ORDER BY q.chapter_num"
    ))
    .bind(title_id)
    .bind(allow_explicit as i64)
    .fetch_all(pool)
    .await
}

pub async fn clear_completed(pool: &SqlitePool) -> sqlx::Result<()> {
    sqlx::query("DELETE FROM download_queue WHERE status IN ('done', 'cancelled', 'error')")
        .execute(pool)
        .await?;
    Ok(())
}

#[derive(FromRow)]
pub struct QueueOwnership {
    pub queued_by: Option<String>,
    pub status: String,
}

pub async fn get_ownership(pool: &SqlitePool, id: &str) -> sqlx::Result<Option<QueueOwnership>> {
    sqlx::query_as("SELECT queued_by, status FROM download_queue WHERE id = ?")
        .bind(id)
        .fetch_optional(pool)
        .await
}

/// Deletes the item unless it's `downloading`, in which case it's
/// soft-cancelled instead (mirrors `Api/Queue.cs`'s `RemoveFromQueue`).
pub async fn remove_or_cancel(pool: &SqlitePool, id: &str) -> sqlx::Result<()> {
    let deleted =
        sqlx::query("DELETE FROM download_queue WHERE id = ? AND status != 'downloading'")
            .bind(id)
            .execute(pool)
            .await?
            .rows_affected();

    if deleted == 0 {
        sqlx::query("UPDATE download_queue SET status = 'cancelled', updated_at = ? WHERE id = ?")
            .bind(ef_timestamp_now())
            .bind(id)
            .execute(pool)
            .await?;
    }
    Ok(())
}

/// Admin always sees/manages explicit content regardless of the
/// `allow_explicit` flag — pure, directly unit-testable (port of
/// `Queue.cs`'s `IsAllowedExplicit` overload).
pub fn is_allowed_explicit(role: &str, allow_explicit: bool) -> bool {
    allow_explicit || role == "admin"
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn explicit_allowed_when_flag_set() {
        assert!(is_allowed_explicit("member", true));
    }

    #[test]
    fn explicit_allowed_for_admin_regardless_of_flag() {
        assert!(is_allowed_explicit("admin", false));
    }

    #[test]
    fn explicit_denied_for_member_without_flag() {
        assert!(!is_allowed_explicit("member", false));
    }
}
