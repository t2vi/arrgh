//! `titles` + `title_sources`/`title_aliases`/`user_titles`/
//! `user_title_settings`/`sync_log`/`sync_warnings` access (ADR 0033, S4
//! #126). Port of the data half of `Api/Titles.cs`. No sqlx migrations —
//! same deferral as `users.rs`/`sources.rs`: .NET's EF migrations still own
//! the schema until cutover (S10). `title_meta` (the cover-cache table)
//! isn't touched here — that's `crate::discover`'s (S6 #128).
//!
//! Chapter-sync itself (the plugin-host fetch) lives in `crate::chapters`
//! (S5 #127); Discover's `AddManga`/`MatchSourcesAsync` DB access
//! (insert/dedup/alias helpers below) lives here since it's all `titles`-
//! table shaped, called from `crate::discover` and `src/api/discover.rs`
//! (S6 #128). `/api/titles` flips together with `chapters`/`progress`/
//! `queue` in `docker/nginx.conf` — they share hot tables and the ADR
//! moved that block as one unit once S7 (#129) landed.

use serde::Serialize;
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
pub struct TitleListItem {
    pub id: String,
    pub title: String,
    pub description: Option<String>,
    pub cover_url: Option<String>,
    pub status: String,
    pub is_local: bool,
    pub local_path: Option<String>,
    pub author: Option<String>,
    pub year: Option<i64>,
    pub tags: Option<String>,
    pub sync_status: String,
    pub content_type: String,
    pub is_explicit: bool,
    pub auto_download: Option<bool>,
    pub reader_mode: Option<String>,
    pub download_dir: Option<String>,
    pub created_at: String,
    pub updated_at: String,
    pub total_chapters: i64,
    pub downloaded_chapters: i64,
    pub chapters_read: i64,
    pub has_sync_warnings: bool,
}

// Two `?` placeholders here (reader_mode + chapters_read subqueries) — every
// caller must `.bind(user_id)` twice, first, before any WHERE-clause binds.
const TITLE_SELECT: &str = "SELECT \
    t.id, t.title, t.description, t.cover_url, t.status, \
    (NOT EXISTS(SELECT 1 FROM title_sources ts WHERE ts.title_id = t.id)) AS is_local, \
    t.local_path, t.author, t.year, t.tags, t.sync_status, t.content_type, \
    t.is_explicit, t.auto_download, \
    (SELECT uts.reader_mode FROM user_title_settings uts WHERE uts.title_id = t.id AND uts.user_id = ?) AS reader_mode, \
    t.download_dir, t.created_at, t.updated_at, \
    (SELECT COUNT(*) FROM chapters c WHERE c.title_id = t.id) AS total_chapters, \
    (SELECT COUNT(*) FROM chapters c WHERE c.title_id = t.id AND c.downloaded = 1) AS downloaded_chapters, \
    (SELECT COUNT(*) FROM read_progress rp JOIN chapters c2 ON c2.id = rp.chapter_id WHERE rp.user_id = ? AND rp.completed = 1 AND c2.title_id = t.id) AS chapters_read, \
    (EXISTS(SELECT 1 FROM sync_warnings sw WHERE sw.title_id = t.id)) AS has_sync_warnings \
    FROM titles t";

pub async fn get_title(
    pool: &SqlitePool,
    id: &str,
    user_id: &str,
    allow_explicit: bool,
) -> sqlx::Result<Option<TitleListItem>> {
    let sql = format!(
        "{TITLE_SELECT} WHERE t.id = ? \
         AND EXISTS(SELECT 1 FROM user_titles ut WHERE ut.user_id = ? AND ut.title_id = t.id) \
         AND (t.is_explicit = 0 OR ? = 1)"
    );
    sqlx::query_as(&sql)
        .bind(user_id)
        .bind(user_id)
        .bind(id)
        .bind(user_id)
        .bind(allow_explicit as i64)
        .fetch_optional(pool)
        .await
}

/// Unfiltered fetch by id — no ownership or explicit gate. Port of
/// `Titles.FetchTitleAsync`, used by Discover's `AddManga` to return the
/// just-created/updated title regardless of explicit status.
pub async fn fetch_title(
    pool: &SqlitePool,
    id: &str,
    user_id: &str,
) -> sqlx::Result<Option<TitleListItem>> {
    let sql = format!("{TITLE_SELECT} WHERE t.id = ?");
    sqlx::query_as(&sql)
        .bind(user_id)
        .bind(user_id)
        .bind(id)
        .fetch_optional(pool)
        .await
}

#[derive(Default)]
pub struct ListParams<'a> {
    pub search: Option<&'a str>,
    pub content_types: Option<Vec<&'a str>>,
    pub statuses: Option<Vec<&'a str>>,
}

/// Bind order for the returned fragment: `user_id`, `allow_explicit as i64`,
/// then `%search%` if `search.is_some()`, then one bind per `content_types`
/// entry if `Some`, then one bind per `statuses` entry if `Some`.
fn where_clause(p: &ListParams) -> String {
    let mut sql = String::from(
        "WHERE EXISTS(SELECT 1 FROM user_titles ut WHERE ut.user_id = ? AND ut.title_id = t.id) \
         AND (t.is_explicit = 0 OR ? = 1)",
    );
    if p.search.is_some() {
        sql.push_str(" AND t.title LIKE ?");
    }
    if let Some(types) = &p.content_types {
        sql.push_str(&format!(
            " AND t.content_type IN ({})",
            vec!["?"; types.len()].join(",")
        ));
    }
    if let Some(statuses) = &p.statuses {
        sql.push_str(&format!(
            " AND t.status IN ({})",
            vec!["?"; statuses.len()].join(",")
        ));
    }
    sql
}

pub async fn count_titles(
    pool: &SqlitePool,
    user_id: &str,
    allow_explicit: bool,
    p: &ListParams<'_>,
) -> sqlx::Result<i64> {
    let sql = format!("SELECT COUNT(*) FROM titles t {}", where_clause(p));
    let mut q = sqlx::query_scalar(&sql)
        .bind(user_id)
        .bind(allow_explicit as i64);
    if let Some(s) = p.search {
        q = q.bind(format!("%{s}%"));
    }
    if let Some(types) = &p.content_types {
        for t in types {
            q = q.bind(*t);
        }
    }
    if let Some(statuses) = &p.statuses {
        for s in statuses {
            q = q.bind(*s);
        }
    }
    q.fetch_one(pool).await
}

pub async fn list_titles(
    pool: &SqlitePool,
    user_id: &str,
    allow_explicit: bool,
    p: &ListParams<'_>,
    order_by: &str,
    limit: i64,
    offset: i64,
) -> sqlx::Result<Vec<TitleListItem>> {
    let sql = format!(
        "{TITLE_SELECT} {} ORDER BY {order_by} LIMIT ? OFFSET ?",
        where_clause(p)
    );
    let mut q = sqlx::query_as::<_, TitleListItem>(&sql)
        .bind(user_id)
        .bind(user_id)
        .bind(user_id)
        .bind(allow_explicit as i64);
    if let Some(s) = p.search {
        q = q.bind(format!("%{s}%"));
    }
    if let Some(types) = &p.content_types {
        for t in types {
            q = q.bind(*t);
        }
    }
    if let Some(statuses) = &p.statuses {
        for s in statuses {
            q = q.bind(*s);
        }
    }
    q = q.bind(limit).bind(offset);
    q.fetch_all(pool).await
}

#[derive(FromRow)]
pub struct NewReleaseItem {
    pub chapter_id: String,
    pub chapter_number: f64,
    pub chapter_title: Option<String>,
    pub chapter_created_at: String,
    pub downloaded: bool,
    pub manga_id: String,
    pub manga_title: String,
    pub cover_url: Option<String>,
}

pub async fn new_releases(
    pool: &SqlitePool,
    user_id: &str,
    allow_explicit: bool,
) -> sqlx::Result<Vec<NewReleaseItem>> {
    sqlx::query_as(
        "SELECT c.id AS chapter_id, c.number AS chapter_number, c.title AS chapter_title, \
         c.created_at AS chapter_created_at, c.downloaded AS downloaded, \
         t.id AS manga_id, t.title AS manga_title, t.cover_url AS cover_url \
         FROM chapters c JOIN titles t ON t.id = c.title_id \
         WHERE c.is_new = 1 \
         AND EXISTS(SELECT 1 FROM user_titles ut WHERE ut.user_id = ? AND ut.title_id = t.id) \
         AND (t.is_explicit = 0 OR ? = 1) \
         ORDER BY c.created_at DESC LIMIT 30",
    )
    .bind(user_id)
    .bind(allow_explicit as i64)
    .fetch_all(pool)
    .await
}

// ── ownership / lifecycle ───────────────────────────────────────────────────

pub async fn is_owned(pool: &SqlitePool, user_id: &str, title_id: &str) -> sqlx::Result<bool> {
    let n: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM user_titles WHERE user_id = ? AND title_id = ?")
            .bind(user_id)
            .bind(title_id)
            .fetch_one(pool)
            .await?;
    Ok(n > 0)
}

/// Content type for a title, gated on ownership (used by the sync endpoint,
/// mirrors `db.Titles.FirstOrDefaultAsync(t => t.Id==id && t.UserTitles.Any(...))`).
pub async fn owned_content_type(
    pool: &SqlitePool,
    user_id: &str,
    title_id: &str,
) -> sqlx::Result<Option<String>> {
    sqlx::query_scalar(
        "SELECT content_type FROM titles \
         WHERE id = ? AND EXISTS(SELECT 1 FROM user_titles ut WHERE ut.user_id = ? AND ut.title_id = titles.id)",
    )
    .bind(title_id)
    .bind(user_id)
    .fetch_optional(pool)
    .await
}

pub async fn get_content_type(pool: &SqlitePool, id: &str) -> sqlx::Result<Option<String>> {
    sqlx::query_scalar("SELECT content_type FROM titles WHERE id = ?")
        .bind(id)
        .fetch_optional(pool)
        .await
}

pub async fn get_mangaupdates_id(pool: &SqlitePool, id: &str) -> sqlx::Result<Option<String>> {
    sqlx::query_scalar::<_, Option<String>>("SELECT mangaupdates_id FROM titles WHERE id = ?")
        .bind(id)
        .fetch_optional(pool)
        .await
        .map(|o| o.flatten())
}

/// Only writes when the column is currently `NULL` — mirrors
/// `RefreshMetadata`'s `if (title.CoverUrl is null) title.CoverUrl = ...`.
pub async fn set_cover_url_if_absent(pool: &SqlitePool, id: &str, v: &str) -> sqlx::Result<()> {
    sqlx::query("UPDATE titles SET cover_url = ? WHERE id = ? AND cover_url IS NULL")
        .bind(v)
        .bind(id)
        .execute(pool)
        .await?;
    Ok(())
}

/// `true` if a `user_titles` row was actually deleted (caller maps absence to 404).
pub async fn remove_user_title(
    pool: &SqlitePool,
    user_id: &str,
    title_id: &str,
) -> sqlx::Result<bool> {
    let res = sqlx::query("DELETE FROM user_titles WHERE user_id = ? AND title_id = ?")
        .bind(user_id)
        .bind(title_id)
        .execute(pool)
        .await?;
    Ok(res.rows_affected() > 0)
}

pub async fn count_owners(pool: &SqlitePool, title_id: &str) -> sqlx::Result<i64> {
    sqlx::query_scalar("SELECT COUNT(*) FROM user_titles WHERE title_id = ?")
        .bind(title_id)
        .fetch_one(pool)
        .await
}

pub async fn chapter_local_paths(pool: &SqlitePool, title_id: &str) -> sqlx::Result<Vec<String>> {
    sqlx::query_scalar(
        "SELECT local_path FROM chapters WHERE title_id = ? AND local_path IS NOT NULL",
    )
    .bind(title_id)
    .fetch_all(pool)
    .await
}

pub async fn cover_url(pool: &SqlitePool, title_id: &str) -> sqlx::Result<Option<String>> {
    sqlx::query_scalar::<_, Option<String>>("SELECT cover_url FROM titles WHERE id = ?")
        .bind(title_id)
        .fetch_optional(pool)
        .await
        .map(|o| o.flatten())
}

/// `true` if a `titles` row was actually deleted. Cascades (chapters,
/// title_sources, title_aliases, sync_log, sync_warnings, user_titles,
/// user_title_settings, read_progress) rely on `PRAGMA foreign_keys=ON`,
/// set once in `state::connect_db`.
pub async fn delete_title(pool: &SqlitePool, id: &str) -> sqlx::Result<bool> {
    let res = sqlx::query("DELETE FROM titles WHERE id = ?")
        .bind(id)
        .execute(pool)
        .await?;
    Ok(res.rows_affected() > 0)
}

// ── patch ────────────────────────────────────────────────────────────────

pub async fn update_auto_download(pool: &SqlitePool, id: &str, v: bool) -> sqlx::Result<()> {
    sqlx::query("UPDATE titles SET auto_download = ? WHERE id = ?")
        .bind(v)
        .bind(id)
        .execute(pool)
        .await?;
    Ok(())
}

pub async fn upsert_reader_mode(
    pool: &SqlitePool,
    user_id: &str,
    title_id: &str,
    reader_mode: &str,
) -> sqlx::Result<()> {
    sqlx::query(
        "INSERT INTO user_title_settings (user_id, title_id, reader_mode) VALUES (?, ?, ?) \
         ON CONFLICT(user_id, title_id) DO UPDATE SET reader_mode = excluded.reader_mode",
    )
    .bind(user_id)
    .bind(title_id)
    .bind(reader_mode)
    .execute(pool)
    .await?;
    Ok(())
}

pub async fn update_download_dir(pool: &SqlitePool, id: &str, v: Option<&str>) -> sqlx::Result<()> {
    sqlx::query("UPDATE titles SET download_dir = ? WHERE id = ?")
        .bind(v)
        .bind(id)
        .execute(pool)
        .await?;
    Ok(())
}

pub async fn update_is_explicit(pool: &SqlitePool, id: &str, v: bool) -> sqlx::Result<()> {
    sqlx::query("UPDATE titles SET is_explicit = ? WHERE id = ?")
        .bind(v)
        .bind(id)
        .execute(pool)
        .await?;
    Ok(())
}

pub async fn update_cover_url(pool: &SqlitePool, id: &str, v: Option<&str>) -> sqlx::Result<()> {
    sqlx::query("UPDATE titles SET cover_url = ? WHERE id = ?")
        .bind(v)
        .bind(id)
        .execute(pool)
        .await?;
    Ok(())
}

pub async fn update_content_type(pool: &SqlitePool, id: &str, v: &str) -> sqlx::Result<()> {
    sqlx::query("UPDATE titles SET content_type = ? WHERE id = ?")
        .bind(v)
        .bind(id)
        .execute(pool)
        .await?;
    Ok(())
}

pub async fn update_sync_status(pool: &SqlitePool, id: &str, status: &str) -> sqlx::Result<()> {
    sqlx::query("UPDATE titles SET sync_status = ? WHERE id = ?")
        .bind(status)
        .bind(id)
        .execute(pool)
        .await?;
    Ok(())
}

// ── sync log / sync warnings / source links ─────────────────────────────────

#[derive(FromRow, Serialize)]
pub struct SyncLogEntry {
    pub id: String,
    pub message: String,
    pub created_at: String,
}

pub async fn list_sync_log(pool: &SqlitePool, title_id: &str) -> sqlx::Result<Vec<SyncLogEntry>> {
    sqlx::query_as(
        "SELECT id, message, created_at FROM sync_log WHERE title_id = ? ORDER BY created_at",
    )
    .bind(title_id)
    .fetch_all(pool)
    .await
}

pub async fn clear_sync_log(pool: &SqlitePool, title_id: &str) -> sqlx::Result<()> {
    sqlx::query("DELETE FROM sync_log WHERE title_id = ?")
        .bind(title_id)
        .execute(pool)
        .await?;
    Ok(())
}

/// Best-effort — mirrors `Api/Titles.cs`'s `AppendSyncLogAsync`, which
/// swallows every error so a logging failure never breaks a sync run.
pub async fn append_sync_log(pool: &SqlitePool, title_id: &str, message: &str) {
    let _ =
        sqlx::query("INSERT INTO sync_log (id, title_id, message, created_at) VALUES (?, ?, ?, ?)")
            .bind(uuid::Uuid::new_v4().to_string())
            .bind(title_id)
            .bind(message)
            .bind(ef_timestamp_now())
            .execute(pool)
            .await;
}

pub async fn clear_sync_warnings(pool: &SqlitePool, title_id: &str) -> sqlx::Result<()> {
    sqlx::query("DELETE FROM sync_warnings WHERE title_id = ?")
        .bind(title_id)
        .execute(pool)
        .await?;
    Ok(())
}

pub async fn title_source_links(
    pool: &SqlitePool,
    title_id: &str,
) -> sqlx::Result<Vec<(String, String)>> {
    sqlx::query_as("SELECT source, source_id FROM title_sources WHERE title_id = ?")
        .bind(title_id)
        .fetch_all(pool)
        .await
}

/// Best-effort — mirrors `AppendSyncWarningAsync`, which swallows every
/// error so a logging failure never breaks source-matching.
pub async fn append_sync_warning(
    pool: &SqlitePool,
    title_id: &str,
    plugin_id: &str,
    message: &str,
) {
    let _ = sqlx::query(
        "INSERT INTO sync_warnings (id, title_id, plugin_id, message, created_at) VALUES (?, ?, ?, ?, ?)",
    )
    .bind(uuid::Uuid::new_v4().to_string())
    .bind(title_id)
    .bind(plugin_id)
    .bind(message)
    .bind(ef_timestamp_now())
    .execute(pool)
    .await;
}

pub async fn has_any_source_link(pool: &SqlitePool, title_id: &str) -> sqlx::Result<bool> {
    let n: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM title_sources WHERE title_id = ?")
        .bind(title_id)
        .fetch_one(pool)
        .await?;
    Ok(n > 0)
}

pub async fn has_title_source_for(
    pool: &SqlitePool,
    title_id: &str,
    source: &str,
) -> sqlx::Result<bool> {
    let n: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM title_sources WHERE title_id = ? AND source = ?")
            .bind(title_id)
            .bind(source)
            .fetch_one(pool)
            .await?;
    Ok(n > 0)
}

pub async fn insert_title_source(
    pool: &SqlitePool,
    title_id: &str,
    source: &str,
    source_id: &str,
) -> sqlx::Result<()> {
    sqlx::query(
        "INSERT INTO title_sources (id, title_id, source, source_id, discovered_at) VALUES (?, ?, ?, ?, ?)",
    )
    .bind(uuid::Uuid::new_v4().to_string())
    .bind(title_id)
    .bind(source)
    .bind(source_id)
    .bind(ef_timestamp_now())
    .execute(pool)
    .await?;
    Ok(())
}

pub async fn list_title_aliases(pool: &SqlitePool, title_id: &str) -> sqlx::Result<Vec<String>> {
    sqlx::query_scalar("SELECT alias FROM title_aliases WHERE title_id = ?")
        .bind(title_id)
        .fetch_all(pool)
        .await
}

pub async fn clear_title_aliases(pool: &SqlitePool, title_id: &str) -> sqlx::Result<()> {
    sqlx::query("DELETE FROM title_aliases WHERE title_id = ?")
        .bind(title_id)
        .execute(pool)
        .await?;
    Ok(())
}

pub async fn insert_title_alias(
    pool: &SqlitePool,
    title_id: &str,
    alias: &str,
) -> sqlx::Result<()> {
    sqlx::query("INSERT INTO title_aliases (id, title_id, alias) VALUES (?, ?, ?)")
        .bind(uuid::Uuid::new_v4().to_string())
        .bind(title_id)
        .bind(alias)
        .execute(pool)
        .await?;
    Ok(())
}

// ── Discover / AddManga (S6 #128) ───────────────────────────────────────────

pub async fn find_id_by_metadata_source(
    pool: &SqlitePool,
    metadata_source: &str,
    metadata_source_id: &str,
) -> sqlx::Result<Option<String>> {
    sqlx::query_scalar("SELECT id FROM titles WHERE metadata_source = ? AND metadata_source_id = ?")
        .bind(metadata_source)
        .bind(metadata_source_id)
        .fetch_optional(pool)
        .await
}

pub async fn find_id_by_mangaupdates_id(
    pool: &SqlitePool,
    mu_id: &str,
) -> sqlx::Result<Option<String>> {
    sqlx::query_scalar("SELECT id FROM titles WHERE mangaupdates_id = ?")
        .bind(mu_id)
        .fetch_optional(pool)
        .await
}

/// User-scoped variant of `find_id_by_metadata_source` — feeds
/// `CheckInLibraryAsync`'s 1st tier (unlike `AddManga`'s dedup check, which
/// is deliberately global across users).
pub async fn find_owned_id_by_metadata_source(
    pool: &SqlitePool,
    user_id: &str,
    metadata_source: &str,
    metadata_source_id: &str,
) -> sqlx::Result<Option<String>> {
    sqlx::query_scalar(
        "SELECT t.id FROM titles t \
         WHERE t.metadata_source = ? AND t.metadata_source_id = ? \
         AND EXISTS(SELECT 1 FROM user_titles ut WHERE ut.user_id = ? AND ut.title_id = t.id)",
    )
    .bind(metadata_source)
    .bind(metadata_source_id)
    .bind(user_id)
    .fetch_optional(pool)
    .await
}

/// User-scoped variant of `find_id_by_mangaupdates_id` — feeds
/// `CheckInLibraryAsync`'s 2nd (legacy) tier.
pub async fn find_owned_id_by_mangaupdates_id(
    pool: &SqlitePool,
    user_id: &str,
    mu_id: &str,
) -> sqlx::Result<Option<String>> {
    sqlx::query_scalar(
        "SELECT t.id FROM titles t WHERE t.mangaupdates_id = ? \
         AND EXISTS(SELECT 1 FROM user_titles ut WHERE ut.user_id = ? AND ut.title_id = t.id)",
    )
    .bind(mu_id)
    .bind(user_id)
    .fetch_optional(pool)
    .await
}

/// Merge new metadata into an existing title without clobbering what's
/// already there — mirrors the `?? ` (coalesce) `ExecuteUpdateAsync` in
/// `AddManga`.
pub async fn coalesce_update_title(
    pool: &SqlitePool,
    id: &str,
    description: Option<&str>,
    author: Option<&str>,
    year: Option<i64>,
    tags: Option<&str>,
) -> sqlx::Result<()> {
    sqlx::query(
        "UPDATE titles SET \
             description = COALESCE(description, ?), \
             author      = COALESCE(author, ?), \
             year        = COALESCE(year, ?), \
             tags        = COALESCE(tags, ?) \
         WHERE id = ?",
    )
    .bind(description)
    .bind(author)
    .bind(year)
    .bind(tags)
    .bind(id)
    .execute(pool)
    .await?;
    Ok(())
}

#[allow(clippy::too_many_arguments)]
pub struct NewTitle<'a> {
    pub id: &'a str,
    pub mangaupdates_id: Option<&'a str>,
    pub metadata_source: Option<&'a str>,
    pub metadata_source_id: Option<&'a str>,
    pub title: &'a str,
    pub description: Option<&'a str>,
    pub cover_url: Option<&'a str>,
    pub status: &'a str,
    pub author: Option<&'a str>,
    pub year: Option<i64>,
    pub tags: Option<&'a str>,
    pub content_type: &'a str,
    pub is_explicit: bool,
}

pub async fn insert_title(pool: &SqlitePool, t: &NewTitle<'_>) -> sqlx::Result<()> {
    let now = ef_timestamp_now();
    sqlx::query(
        "INSERT INTO titles \
             (id, mangaupdates_id, metadata_source, metadata_source_id, title, description, \
              cover_url, status, author, year, tags, sync_status, content_type, is_explicit, \
              created_at, updated_at) \
         VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, 'syncing', ?, ?, ?, ?)",
    )
    .bind(t.id)
    .bind(t.mangaupdates_id)
    .bind(t.metadata_source)
    .bind(t.metadata_source_id)
    .bind(t.title)
    .bind(t.description)
    .bind(t.cover_url)
    .bind(t.status)
    .bind(t.author)
    .bind(t.year)
    .bind(t.tags)
    .bind(t.content_type)
    .bind(t.is_explicit)
    .bind(&now)
    .bind(&now)
    .execute(pool)
    .await?;
    Ok(())
}

/// Idempotent — no-ops if `user_id` already owns `title_id`.
pub async fn insert_user_title_if_absent(
    pool: &SqlitePool,
    user_id: &str,
    title_id: &str,
) -> sqlx::Result<()> {
    sqlx::query("INSERT OR IGNORE INTO user_titles (user_id, title_id, added_at) VALUES (?, ?, ?)")
        .bind(user_id)
        .bind(title_id)
        .bind(ef_timestamp_now())
        .execute(pool)
        .await?;
    Ok(())
}

/// `(id, title)` for every title of `content_type` owned by `user_id` —
/// feeds Discover's normalized-title library check (3rd tier of
/// `CheckInLibraryAsync`; done in Rust rather than SQL since normalization
/// isn't expressible as a SQL predicate).
pub async fn list_owned_by_content_type(
    pool: &SqlitePool,
    user_id: &str,
    content_type: &str,
) -> sqlx::Result<Vec<(String, String)>> {
    sqlx::query_as(
        "SELECT t.id, t.title FROM titles t \
         WHERE t.content_type = ? AND EXISTS(SELECT 1 FROM user_titles ut WHERE ut.user_id = ? AND ut.title_id = t.id)",
    )
    .bind(content_type)
    .bind(user_id)
    .fetch_all(pool)
    .await
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn where_clause_base_only() {
        let sql = where_clause(&ListParams::default());
        assert!(sql.contains("user_titles"));
        assert!(!sql.contains("LIKE"));
        assert!(!sql.contains("IN ("));
    }

    #[test]
    fn where_clause_with_search_and_filters() {
        let p = ListParams {
            search: Some("naruto"),
            content_types: Some(vec!["manga", "manhwa"]),
            statuses: Some(vec!["ongoing"]),
        };
        let sql = where_clause(&p);
        assert!(sql.contains("t.title LIKE ?"));
        assert!(sql.contains("t.content_type IN (?,?)"));
        assert!(sql.contains("t.status IN (?)"));
    }
}
