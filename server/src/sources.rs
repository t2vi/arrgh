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

// ── plugins (ADR 0033, S9 #131) ──────────────────────────────────────────

pub async fn exists_by_base_url(pool: &SqlitePool, base_url: &str) -> sqlx::Result<bool> {
    sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM external_sources WHERE base_url = ?)")
        .bind(base_url)
        .fetch_one(pool)
        .await
}

#[derive(FromRow)]
pub struct SourceByKey {
    pub id: String,
    pub is_community: bool,
}

pub async fn find_by_source_key(
    pool: &SqlitePool,
    source_key: &str,
) -> sqlx::Result<Option<SourceByKey>> {
    sqlx::query_as("SELECT id, is_community FROM external_sources WHERE source_key = ?")
        .bind(source_key)
        .fetch_optional(pool)
        .await
}

/// Inserts a newly-installed community plugin as an `external_sources` row.
///
/// `source_key = plugin_id` — unlike `Api/Plugins.cs`'s `InstallPlugin`,
/// which never sets `SourceKey` on install (a bug: bundled sources get it
/// from `Program.cs` seed data, but an installed one would be permanently
/// undeletable via `DeletePlugin`'s `SourceKey` lookup and invisible to
/// Discover's source-matching, which filters `source_key IS NOT NULL`).
/// Fixed here per user decision (2026-09-15) rather than ported bug-for-bug.
pub async fn insert_community_source(
    pool: &SqlitePool,
    id: &str,
    plugin_id: &str,
    name: &str,
    base_url: &str,
    content_types: &str,
    default_explicit: bool,
) -> sqlx::Result<()> {
    sqlx::query(
        "INSERT INTO external_sources \
             (id, name, base_url, content_types, enabled, created_at, is_community, priority, source_key, default_explicit) \
         VALUES (?, ?, ?, ?, 1, datetime('now'), 1, 100, ?, ?)",
    )
    .bind(id)
    .bind(name)
    .bind(base_url)
    .bind(content_types)
    .bind(plugin_id)
    .bind(default_explicit)
    .execute(pool)
    .await?;
    Ok(())
}
