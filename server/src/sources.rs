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

/// `(source_key, priority)` for every enabled source matching `content_type`
/// — feeds Discover's `match_sources` (ADR 0033, S6 #128), port of
/// `MatchSourcesAsync`'s source query. `include_hentai` widens the match to
/// `%hentai%` too (the explicit-manga branch: MU sometimes classifies
/// hentai doujinshi as "manga").
pub async fn matching_for_content_type(
    pool: &SqlitePool,
    content_type: &str,
    include_hentai: bool,
) -> sqlx::Result<Vec<(String, i64)>> {
    let sql = if include_hentai {
        "SELECT source_key, priority FROM external_sources \
         WHERE enabled = 1 AND source_key IS NOT NULL \
         AND (content_types LIKE '%manga%' OR content_types LIKE '%hentai%') \
         ORDER BY priority"
    } else {
        "SELECT source_key, priority FROM external_sources \
         WHERE enabled = 1 AND source_key IS NOT NULL AND content_types LIKE ? \
         ORDER BY priority"
    };
    let mut q = sqlx::query_as(sql);
    if !include_hentai {
        q = q.bind(format!("%{content_type}%"));
    }
    q.fetch_all(pool).await
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
