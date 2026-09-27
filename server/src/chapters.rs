//! `chapters` + `chapter_sources` access, plus the shared chapter-sync
//! routine (ADR 0033, S5 #127). Port of `Api/Chapters.cs`'s data half and
//! `Api/ChapterSync.cs` in full — this is the real plugin-host fetch, not a
//! stub; it replaces the S4-era `titles::sync_from_source` stub.
//!
//! `download_queue` (`queue_download`) is part of `Chapters.cs` even though
//! the queue *processor* (`DownloaderService`) is S7 — the enqueue is a
//! self-contained SQL upsert with no dependency on the worker existing yet.

use std::collections::{HashMap, HashSet};

use serde::Deserialize;
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
pub struct ChapterRow {
    pub id: String,
    pub title_id: String,
    pub title: Option<String>,
    pub number: f64,
    pub volume: Option<f64>,
    pub local_path: Option<String>,
    pub page_count: i64,
    pub downloaded: bool,
    pub has_sources: bool,
    pub chapter_format: String,
    pub created_at: String,
}

const CHAPTER_SELECT: &str = "SELECT \
    c.id, c.title_id, c.title, c.number, c.volume, c.local_path, c.page_count, c.downloaded, \
    (EXISTS(SELECT 1 FROM chapter_sources cs WHERE cs.chapter_id = c.id)) AS has_sources, \
    c.chapter_format, c.created_at \
    FROM chapters c JOIN titles t ON t.id = c.title_id";

pub async fn list_chapters(
    pool: &SqlitePool,
    title_id: &str,
    allow_explicit: bool,
) -> sqlx::Result<Vec<ChapterRow>> {
    sqlx::query_as(&format!(
        "{CHAPTER_SELECT} WHERE c.title_id = ? AND (t.is_explicit = 0 OR ? = 1) ORDER BY c.number"
    ))
    .bind(title_id)
    .bind(allow_explicit as i64)
    .fetch_all(pool)
    .await
}

pub async fn get_chapter(
    pool: &SqlitePool,
    id: &str,
    allow_explicit: bool,
) -> sqlx::Result<Option<ChapterRow>> {
    sqlx::query_as(&format!(
        "{CHAPTER_SELECT} WHERE c.id = ? AND (t.is_explicit = 0 OR ? = 1)"
    ))
    .bind(id)
    .bind(allow_explicit as i64)
    .fetch_optional(pool)
    .await
}

#[derive(FromRow)]
pub struct ChapterTextInfo {
    pub local_path: Option<String>,
    pub downloaded: bool,
    pub chapter_format: String,
}

pub async fn chapter_text_info(
    pool: &SqlitePool,
    id: &str,
    allow_explicit: bool,
) -> sqlx::Result<Option<ChapterTextInfo>> {
    sqlx::query_as(
        "SELECT c.local_path, c.downloaded, c.chapter_format FROM chapters c \
         JOIN titles t ON t.id = c.title_id \
         WHERE c.id = ? AND (t.is_explicit = 0 OR ? = 1)",
    )
    .bind(id)
    .bind(allow_explicit as i64)
    .fetch_optional(pool)
    .await
}

#[derive(FromRow)]
pub struct QueueCandidate {
    pub id: String,
    pub number: f64,
    pub manga_title: String,
}

/// Downloadable candidate: not already downloaded, has at least one
/// `chapter_sources` link, and passes the explicit gate.
pub async fn queue_candidate(
    pool: &SqlitePool,
    id: &str,
    allow_explicit: bool,
) -> sqlx::Result<Option<QueueCandidate>> {
    sqlx::query_as(
        "SELECT c.id, c.number, t.title AS manga_title FROM chapters c \
         JOIN titles t ON t.id = c.title_id \
         WHERE c.id = ? AND c.downloaded = 0 \
         AND EXISTS(SELECT 1 FROM chapter_sources cs WHERE cs.chapter_id = c.id) \
         AND (t.is_explicit = 0 OR ? = 1)",
    )
    .bind(id)
    .bind(allow_explicit as i64)
    .fetch_optional(pool)
    .await
}

/// Upsert into `download_queue`, keyed on the unique `chapter_id` index.
/// Only takes effect on conflict when the existing row is `error`/`cancelled`
/// — mirrors `Chapters.cs`'s raw-SQL upsert (no EF/SQLx query-builder
/// equivalent for a conditional `ON CONFLICT ... WHERE`).
pub async fn queue_download(
    pool: &SqlitePool,
    chapter_id: &str,
    manga_title: &str,
    chapter_num: f64,
    queued_by: Option<&str>,
) -> sqlx::Result<()> {
    let now = ef_timestamp_now();
    sqlx::query(
        "INSERT INTO download_queue \
             (id, chapter_id, manga_title, chapter_num, status, pages_downloaded, pages_total, created_at, updated_at, queued_by) \
         VALUES (?, ?, ?, ?, 'pending', 0, 0, ?, ?, ?) \
         ON CONFLICT(chapter_id) DO UPDATE SET \
             status = 'pending', \
             error = NULL, \
             queued_by = excluded.queued_by, \
             updated_at = excluded.updated_at \
         WHERE download_queue.status IN ('error', 'cancelled')",
    )
    .bind(uuid::Uuid::new_v4().to_string())
    .bind(chapter_id)
    .bind(manga_title)
    .bind(chapter_num)
    .bind(&now)
    .bind(&now)
    .bind(queued_by)
    .execute(pool)
    .await?;
    Ok(())
}

// ── chapter-sync ─────────────────────────────────────────────────────────

#[derive(Deserialize)]
struct PluginChapter {
    source_id: Option<String>,
    id: Option<String>,
    number: f64,
    volume: Option<f64>,
    title: Option<String>,
}

/// Fetches `{plugin_host_url}/{source}/manga/{source_id}/chapters`, upserts
/// chapters by `(title_id, number)`, then seeds `chapter_sources`
/// (idempotent). Returns the number of chapters the plugin reported.
/// `mark_new` flags inserted chapters `is_new` (Home "New releases"); the
/// caller decides it once per sync run — see `titles::run_sync` (GH #210).
///
/// Errors (network, non-2xx, bad JSON) propagate — the caller (titles.rs's
/// sync orchestration) tracks per-source failure and logs it.
/// A source can renumber a chapter it already reported (e.g. Royal Road
/// "Chapter 2.A" was 2, now 2.1 — spec 030). Chapters dedup by number, so
/// without this the old row lingers next to the new one. For each existing
/// link to one of this source's chapters that now arrives with a different
/// number: move the row to the new number when that's free (keeps downloads
/// and reading progress); otherwise it's a stale duplicate — delete it, unless
/// it's downloaded or another source still links it.
async fn reconcile_renumbered(
    pool: &SqlitePool,
    title_id: &str,
    source: &str,
    plugin_chapters: &[PluginChapter],
) -> anyhow::Result<()> {
    let reported: HashMap<&str, &PluginChapter> = plugin_chapters
        .iter()
        .filter(|pc| pc.number.is_finite())
        .filter_map(|pc| {
            let id = pc.source_id.as_deref().or(pc.id.as_deref())?;
            (!id.is_empty()).then_some((id, pc))
        })
        .collect();
    if reported.is_empty() {
        return Ok(());
    }

    let links: Vec<(String, String, f64, bool, i64)> = sqlx::query_as(
        "SELECT c.id, cs.source_id, c.number, c.downloaded, \
                (SELECT COUNT(*) FROM chapter_sources x WHERE x.chapter_id = c.id) \
         FROM chapter_sources cs JOIN chapters c ON c.id = cs.chapter_id \
         WHERE c.title_id = ? AND cs.source = ?",
    )
    .bind(title_id)
    .bind(source)
    .fetch_all(pool)
    .await?;
    let mut taken: HashSet<u64> =
        sqlx::query_scalar::<_, f64>("SELECT number FROM chapters WHERE title_id = ?")
            .bind(title_id)
            .fetch_all(pool)
            .await?
            .into_iter()
            .map(f64::to_bits)
            .collect();

    for (chapter_id, source_id, number, downloaded, link_count) in links {
        let Some(pc) = reported.get(source_id.as_str()) else {
            continue;
        };
        if pc.number == number {
            continue;
        }
        if taken.insert(pc.number.to_bits()) {
            sqlx::query("UPDATE chapters SET number = ?, title = ? WHERE id = ?")
                .bind(pc.number)
                .bind(&pc.title)
                .bind(&chapter_id)
                .execute(pool)
                .await?;
            taken.remove(&number.to_bits());
        } else if !downloaded && link_count == 1 {
            // chapter_sources / read_progress / download_queue cascade.
            sqlx::query("DELETE FROM chapters WHERE id = ?")
                .bind(&chapter_id)
                .execute(pool)
                .await?;
            taken.remove(&number.to_bits());
        }
    }
    Ok(())
}

#[allow(clippy::too_many_arguments)] // same precedent as titles.rs; a params struct for one caller pair isn't worth it
pub async fn sync_from_source(
    pool: &SqlitePool,
    http: &reqwest::Client,
    plugin_host_url: &str,
    title_id: &str,
    content_type: &str,
    source: &str,
    source_id: &str,
    mark_new: bool,
) -> anyhow::Result<u32> {
    let url = format!(
        "{}/{}/manga/{}/chapters",
        plugin_host_url.trim_end_matches('/'),
        source,
        urlencoding::encode(source_id)
    );
    let resp = http.get(&url).send().await?;
    if !resp.status().is_success() {
        anyhow::bail!("plugin-host {source} returned {}", resp.status());
    }
    let plugin_chapters: Vec<PluginChapter> = resp.json().await?;
    if plugin_chapters.is_empty() {
        return Ok(0);
    }

    let fmt = if content_type == "novel" {
        "text"
    } else {
        "pages"
    };
    let now = ef_timestamp_now();

    reconcile_renumbered(pool, title_id, source, &plugin_chapters).await?;

    let existing: Vec<(String, f64)> =
        sqlx::query_as("SELECT id, number FROM chapters WHERE title_id = ?")
            .bind(title_id)
            .fetch_all(pool)
            .await?;
    let mut by_num: HashMap<u64, String> = existing
        .into_iter()
        .map(|(id, n)| (n.to_bits(), id))
        .collect();

    for pc in &plugin_chapters {
        if pc.number.is_nan() || pc.number.is_infinite() {
            continue;
        }
        let key = pc.number.to_bits();
        if by_num.contains_key(&key) {
            continue;
        }
        let id = uuid::Uuid::new_v4().to_string();
        sqlx::query(
            "INSERT INTO chapters \
                 (id, title_id, number, volume, title, chapter_format, is_new, page_count, downloaded, created_at) \
             VALUES (?, ?, ?, ?, ?, ?, ?, 0, 0, ?)",
        )
        .bind(&id)
        .bind(title_id)
        .bind(pc.number)
        .bind(pc.volume)
        .bind(&pc.title)
        .bind(fmt)
        .bind(mark_new)
        .bind(&now)
        .execute(pool)
        .await?;
        by_num.insert(key, id);
    }

    // Seed chapter_sources — skip pairs that already exist, and dedup
    // within this batch (a plugin listing the same chapter twice).
    let chapter_ids: Vec<&String> = by_num.values().collect();
    let existing_pairs: HashSet<String> = if chapter_ids.is_empty() {
        HashSet::new()
    } else {
        let placeholders = vec!["?"; chapter_ids.len()].join(",");
        let sql = format!(
            "SELECT chapter_id FROM chapter_sources WHERE source = ? AND chapter_id IN ({placeholders})"
        );
        let mut q = sqlx::query_scalar(&sql).bind(source);
        for id in &chapter_ids {
            q = q.bind(id.as_str());
        }
        q.fetch_all(pool).await?.into_iter().collect()
    };

    let mut seen_this_batch = HashSet::new();
    for pc in &plugin_chapters {
        if pc.number.is_nan() || pc.number.is_infinite() {
            continue;
        }
        let Some(chapter_id) = by_num.get(&pc.number.to_bits()) else {
            continue;
        };
        let src_id = pc.source_id.as_deref().or(pc.id.as_deref()).unwrap_or("");
        if src_id.is_empty() {
            continue;
        }
        if !seen_this_batch.insert(chapter_id.clone()) {
            continue;
        }
        if existing_pairs.contains(chapter_id) {
            // The source may now report a different id for the same chapter (e.g.
            // WuxiaWorld's real slug replacing a guessed one, GH #173) — keep the
            // link current so the downloader uses what the source actually serves.
            sqlx::query(
                "UPDATE chapter_sources SET source_id = ? WHERE chapter_id = ? AND source = ? AND source_id <> ?",
            )
            .bind(src_id)
            .bind(chapter_id)
            .bind(source)
            .bind(src_id)
            .execute(pool)
            .await?;
            continue;
        }

        sqlx::query(
            "INSERT INTO chapter_sources (id, chapter_id, source, source_id) VALUES (?, ?, ?, ?)",
        )
        .bind(uuid::Uuid::new_v4().to_string())
        .bind(chapter_id)
        .bind(source)
        .bind(src_id)
        .execute(pool)
        .await?;
    }

    Ok(plugin_chapters.len() as u32)
}
