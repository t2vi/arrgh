//! `external_sources` table access (ADR 0033, S3 #125). Port of the data
//! half of `Api/Sources.cs`. No sqlx migrations, and no seeding — .NET's
//! `Program.cs` still seeds the 9 default sources on boot; Rust only reads
//! what's already there, same deferral as `users.rs`/`settings.rs`.

use sqlx::{FromRow, SqlitePool};

#[derive(FromRow, Clone)]
pub struct SourceRow {
    pub id: String,
    pub name: String,
    pub base_url: String,
    pub api_key: Option<String>,
    pub content_types: String,
    pub enabled: bool,
    pub is_community: bool,
    pub priority: i64,
}

pub async fn list_ordered_by_created_at(pool: &SqlitePool) -> sqlx::Result<Vec<SourceRow>> {
    sqlx::query_as(
        "SELECT id, name, base_url, api_key, content_types, enabled, is_community, priority FROM external_sources ORDER BY created_at",
    )
    .fetch_all(pool)
    .await
}

pub async fn find_by_id(pool: &SqlitePool, id: &str) -> sqlx::Result<Option<SourceRow>> {
    sqlx::query_as(
        "SELECT id, name, base_url, api_key, content_types, enabled, is_community, priority FROM external_sources WHERE id = ?",
    )
    .bind(id)
    .fetch_optional(pool)
    .await
}

pub async fn update_enabled(pool: &SqlitePool, id: &str, enabled: bool) -> sqlx::Result<()> {
    sqlx::query("UPDATE external_sources SET enabled = ? WHERE id = ?")
        .bind(enabled)
        .bind(id)
        .execute(pool)
        .await?;
    Ok(())
}

pub async fn update_priority(pool: &SqlitePool, id: &str, priority: i64) -> sqlx::Result<()> {
    sqlx::query("UPDATE external_sources SET priority = ? WHERE id = ?")
        .bind(priority)
        .bind(id)
        .execute(pool)
        .await?;
    Ok(())
}

/// `true` if a row was actually deleted (caller maps absence to 404).
pub async fn delete(pool: &SqlitePool, id: &str) -> sqlx::Result<bool> {
    let res = sqlx::query("DELETE FROM external_sources WHERE id = ?")
        .bind(id)
        .execute(pool)
        .await?;
    Ok(res.rows_affected() > 0)
}
