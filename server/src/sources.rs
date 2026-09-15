//! `external_sources` table access (ADR 0033, S3 #125). Port of the data
//! half of `Api/Sources.cs`, plus (S10 #132) `Program.cs`'s startup seed of
//! the 9 bundled sources — the only thing left to boot a fresh install.

use sqlx::{FromRow, SqlitePool};
use time::macros::format_description;
use time::OffsetDateTime;

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

// ── startup seed (ADR 0033, S10 #132) ────────────────────────────────────

/// `(source_key, name, content_types, default_explicit, priority)` — exact
/// values `Program.cs` used to seed a fresh install.
const DEFAULT_SOURCES: [(&str, &str, &str, bool, i64); 9] = [
    ("mangadex", "MangaDex", "manga,manhua,one-shot", false, 10),
    ("mangapill", "Mangapill", "manga", false, 20),
    ("toonily", "Toonily", "manhwa", false, 30),
    ("novelfull", "NovelFull", "novel", false, 40),
    ("nhentai", "nhentai", "hentai", true, 50),
    (
        "mangafire",
        "MangaFire",
        "manga,manhwa,manhua,one-shot",
        false,
        60,
    ),
    ("manga18fx", "Manga18fx", "manhwa", true, 75),
    ("wuxiaworld", "WuxiaWorld", "novel", false, 90),
    ("asurascans", "AsuraScans", "manhwa", false, 110),
];

/// Seeds the bundled sources if `external_sources` is empty — port of
/// `Program.cs`'s inline startup block. Called from `run()` on every boot;
/// the emptiness check makes it a no-op past the first.
pub async fn seed_defaults_if_empty(
    pool: &SqlitePool,
    plugin_host_url: &str,
) -> anyhow::Result<()> {
    let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM external_sources")
        .fetch_one(pool)
        .await?;
    if count > 0 {
        return Ok(());
    }

    let fmt =
        format_description!("[year]-[month]-[day] [hour]:[minute]:[second].[subsecond digits:6]");
    let now = OffsetDateTime::now_utc().format(&fmt)?;

    for (source_key, name, content_types, default_explicit, priority) in DEFAULT_SOURCES {
        sqlx::query(
            "INSERT INTO external_sources \
                 (id, name, base_url, content_types, enabled, created_at, is_community, priority, source_key, default_explicit) \
             VALUES (?, ?, ?, ?, 1, ?, 0, ?, ?, ?)",
        )
        .bind(uuid::Uuid::new_v4().to_string())
        .bind(name)
        .bind(plugin_host_url)
        .bind(content_types)
        .bind(&now)
        .bind(priority)
        .bind(source_key)
        .bind(default_explicit)
        .execute(pool)
        .await?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn seed_defaults_inserts_nine_sources_once() {
        let pool = crate::state::connect_db(&format!(
            "{}/arrgh-seed-test-{}.db",
            std::env::temp_dir().display(),
            uuid::Uuid::new_v4()
        ))
        .await
        .unwrap();

        seed_defaults_if_empty(&pool, "http://plugin-host:4000")
            .await
            .unwrap();
        let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM external_sources")
            .fetch_one(&pool)
            .await
            .unwrap();
        assert_eq!(count, 9);

        // Second call is a no-op — table is no longer empty.
        seed_defaults_if_empty(&pool, "http://plugin-host:4000")
            .await
            .unwrap();
        let count_again: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM external_sources")
            .fetch_one(&pool)
            .await
            .unwrap();
        assert_eq!(count_again, 9);
    }
}
