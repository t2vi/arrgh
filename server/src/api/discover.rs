use axum::{
    extract::{Query, State},
    http::StatusCode,
    response::{IntoResponse, Json, Response},
    routing::{get, post},
    Router,
};
use chrono::Utc;
use serde::{Deserialize, Serialize};
use std::path::Path;
use std::sync::Arc;
use std::time::Instant;
use tokio::sync::Mutex;
use uuid::Uuid;

use crate::{auth, db::Manga, indexer::source::{MangaResult, Source}, AppState};
use super::ApiResult;

// ── Trending cache ────────────────────────────────────────────────────────────

#[derive(Clone)]
pub struct CachedTrendingEntry {
    pub source_id: String,
    pub source_name: String,
    pub result: MangaResult,
    pub alternatives: Vec<SourceAlternative>,
}

pub type TrendingCache = Arc<Mutex<Option<(Instant, Vec<CachedTrendingEntry>)>>>;

#[derive(Deserialize)]
pub struct DetailQuery {
    pub source: String,
    pub source_id: String,
    pub title: Option<String>,
}

#[derive(Serialize)]
pub struct MangaDetailResult {
    pub description: Option<String>,
    pub cover_url: Option<String>,
    pub chapter_count: usize,
    pub tags: Option<String>,
}

// ── Title metadata cache helpers ──────────────────────────────────────────────

struct MetaCacheRow {
    cover_local_path: Option<String>,
    cover_cdn_url: Option<String>,
    description: Option<String>,
}

async fn batch_lookup_meta(
    db: &sqlx::SqlitePool,
    keys: &[String],
) -> std::collections::HashMap<String, MetaCacheRow> {
    if keys.is_empty() {
        return std::collections::HashMap::new();
    }
    let placeholders = keys.iter().map(|_| "?").collect::<Vec<_>>().join(",");
    let sql = format!(
        "SELECT title_key, cover_local_path, cover_cdn_url, description FROM title_meta WHERE title_key IN ({})",
        placeholders
    );
    let mut q = sqlx::query(&sql);
    for k in keys {
        q = q.bind(k.as_str());
    }
    match q.fetch_all(db).await {
        Ok(rows) => {
            use sqlx::Row;
            rows.into_iter().map(|row| {
                let key: String = row.get("title_key");
                (key, MetaCacheRow {
                    cover_local_path: row.get("cover_local_path"),
                    cover_cdn_url: row.get("cover_cdn_url"),
                    description: row.get("description"),
                })
            }).collect()
        }
        Err(e) => { tracing::warn!("batch_lookup_meta: {}", e); Default::default() }
    }
}

fn meta_cover_url(key: &str) -> String {
    format!("/api/media/meta-cover?key={}", urlencoding::encode(key))
}

fn enrich_with_cache(
    cache: &std::collections::HashMap<String, MetaCacheRow>,
    results: &mut Vec<SearchResult>,
) {
    for result in results.iter_mut() {
        let key = normalize_title(&result.title);
        if let Some(m) = cache.get(&key) {
            if m.cover_local_path.is_some() {
                result.cover_url = Some(meta_cover_url(&key));
            } else if result.cover_url.is_none() {
                result.cover_url = m.cover_cdn_url.clone();
            }
            if result.description.is_none() {
                result.description = m.description.clone();
            }
        }
    }
}

fn spawn_cover_download(
    db: sqlx::SqlitePool,
    src: Arc<dyn Source>,
    cdn_url: String,
    title_key: String,
    download_dir: String,
) {
    tokio::spawn(async move {
        download_meta_cover(&db, src, cdn_url, title_key, download_dir).await;
    });
}

async fn seed_results_meta(
    db: &sqlx::SqlitePool,
    sources: &[(String, Arc<dyn Source>)],
    download_dir: &str,
    results: &[SearchResult],
    meta_cache: &std::collections::HashMap<String, MetaCacheRow>,
) {
    for result in results {
        let cover_cdn_url = match &result.cover_url {
            Some(u) if !u.starts_with("/api/") => u.clone(),
            _ => continue,
        };
        let key = normalize_title(&result.title);

        if let Some(row) = meta_cache.get(&key) {
            if row.cover_local_path.is_none() {
                let cdn = row.cover_cdn_url.clone().unwrap_or_else(|| cover_cdn_url.clone());
                if let Some((_, src)) = sources.iter().find(|(sid, _)| sid == &result.source) {
                    spawn_cover_download(db.clone(), src.clone(), cdn, key, download_dir.to_string());
                }
            }
            continue;
        }

        store_meta(
            db, &key, &result.source, &result.id,
            result.description.as_deref(),
            Some(&cover_cdn_url),
            result.tags.as_deref(),
            0,
        ).await;

        if let Some((_, src)) = sources.iter().find(|(sid, _)| sid == &result.source) {
            spawn_cover_download(db.clone(), src.clone(), cover_cdn_url, key, download_dir.to_string());
        }
    }
}

async fn store_meta(
    db: &sqlx::SqlitePool,
    key: &str,
    source: &str,
    source_id: &str,
    description: Option<&str>,
    cover_cdn_url: Option<&str>,
    tags: Option<&str>,
    chapter_count: usize,
) {
    let now = Utc::now().to_rfc3339();
    let count = chapter_count as i64;
    if let Err(e) = sqlx::query!(
        r#"INSERT INTO title_meta (title_key, cover_cdn_url, description, tags, chapter_count, source, source_id, fetched_at)
           VALUES (?, ?, ?, ?, ?, ?, ?, ?)
           ON CONFLICT(title_key) DO UPDATE SET
             description  = COALESCE(excluded.description, title_meta.description),
             cover_cdn_url = COALESCE(excluded.cover_cdn_url, title_meta.cover_cdn_url),
             tags         = COALESCE(excluded.tags, title_meta.tags),
             chapter_count = excluded.chapter_count,
             fetched_at   = excluded.fetched_at"#,
        key, cover_cdn_url, description, tags, count, source, source_id, now
    )
    .execute(db)
    .await
    {
        tracing::warn!("store_meta failed for '{}': {}", key, e);
    }
}

async fn download_meta_cover(
    db: &sqlx::SqlitePool,
    src: Arc<dyn Source>,
    cdn_url: String,
    title_key: String,
    download_dir: String,
) {
    let ext = cdn_url.split('?').next().unwrap_or("cover")
        .rsplit('.').next().unwrap_or("jpg");
    let safe = title_key.chars().map(|c| if c.is_alphanumeric() { c } else { '_' }).collect::<String>();
    let cover_path = Path::new(&download_dir)
        .join("_meta")
        .join(format!("{}.{}", safe, ext));

    match src.fetch_cover(&cdn_url).await {
        Ok(bytes) => {
            if let Some(parent) = cover_path.parent() {
                let _ = tokio::fs::create_dir_all(parent).await;
            }
            if tokio::fs::write(&cover_path, &bytes).await.is_ok() {
                let path_str = cover_path.to_string_lossy().to_string();
                let _ = sqlx::query!(
                    "UPDATE title_meta SET cover_local_path = ? WHERE title_key = ?",
                    path_str, title_key
                )
                .execute(db)
                .await;
            }
        }
        Err(e) => tracing::warn!("meta cover download failed for '{}': {}", title_key, e),
    }
}

#[derive(Deserialize)]
pub struct SearchQuery {
    pub q: String,
    pub content_type: Option<String>,
}

#[derive(Serialize, Clone)]
pub struct SourceAlternative {
    pub source: String,
    pub source_name: String,
    pub id: String,
    pub cover_url: Option<String>,
    pub status: String,
}

#[derive(Serialize)]
pub struct SearchResult {
    pub id: String,
    pub source: String,
    pub source_name: String,
    pub title: String,
    pub description: Option<String>,
    pub cover_url: Option<String>,
    pub status: String,
    pub author: Option<String>,
    pub year: Option<i64>,
    pub tags: Option<String>,
    pub content_type: String,
    pub in_library: bool,
    pub library_id: Option<String>,
    pub alternatives: Vec<SourceAlternative>,
}

#[derive(Deserialize)]
pub struct AddMangaRequest {
    pub source: String,
    pub source_id: String,
    pub title: String,
    pub description: Option<String>,
    pub cover_url: Option<String>,
    pub status: String,
    pub author: Option<String>,
    pub year: Option<i64>,
    pub tags: Option<String>,
    #[serde(default = "default_content_type")]
    pub content_type: String,
}

fn default_content_type() -> String { "manga".to_string() }

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/discover", get(search))
        .route("/discover/trending", get(trending))
        .route("/discover/detail", get(detail_meta))
        .route("/discover/add", post(add_manga))
}

// ── Fan-out helpers ───────────────────────────────────────────────────────────

fn normalize_title(title: &str) -> String {
    title
        .chars()
        .map(|c| if c.is_alphanumeric() { c } else { ' ' })
        .collect::<String>()
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .to_lowercase()
}

struct SourceHit {
    source_id: String,
    source_name: String,
    result: crate::indexer::source::MangaResult,
}

async fn fan_out(
    sources: Vec<(String, String, Arc<dyn Source>)>,
    mode: FanOutMode,
) -> Vec<SourceHit> {
    let tasks: Vec<_> = sources
        .into_iter()
        .map(|(sid, sname, src)| {
            let mode = mode.clone();
            tokio::spawn(async move {
                let res = match &mode {
                    FanOutMode::Search(q) => src.search(q).await,
                    FanOutMode::Trending   => src.trending().await,
                };
                match res {
                    Ok(items) => items
                        .into_iter()
                        .map(|r| SourceHit { source_id: sid.clone(), source_name: sname.clone(), result: r })
                        .collect::<Vec<_>>(),
                    Err(e) => {
                        tracing::debug!("{} {} failed: {}", sid, mode.label(), e);
                        vec![]
                    }
                }
            })
        })
        .collect();

    let mut hits = Vec::new();
    for task in tasks {
        if let Ok(batch) = task.await { hits.extend(batch); }
    }
    hits
}

#[derive(Clone)]
enum FanOutMode {
    Search(String),
    Trending,
}

impl FanOutMode {
    fn label(&self) -> &str {
        match self { Self::Search(_) => "search", Self::Trending => "trending" }
    }
}

fn snapshot_sources(
    registry: &std::collections::HashMap<String, Arc<dyn Source>>,
    content_type_filter: Option<&str>,
) -> Vec<(String, String, Arc<dyn Source>)> {
    registry
        .iter()
        .filter(|(_, src)| {
            match content_type_filter {
                None => true,
                Some(ct) => src.content_types().iter().any(|t| t == ct),
            }
        })
        .map(|(id, src)| (id.clone(), src.name().to_string(), src.clone()))
        .collect()
}

/// Merge hits from multiple sources by normalized title.
/// Returns one `SearchResult` per unique title with `alternatives` for secondary sources.
fn interleave_and_limit(hits: Vec<SourceHit>, per_source: usize) -> Vec<SourceHit> {
    let mut source_order: Vec<String> = Vec::new();
    let mut groups: std::collections::HashMap<String, Vec<SourceHit>> = std::collections::HashMap::new();
    for hit in hits {
        if !groups.contains_key(&hit.source_id) {
            source_order.push(hit.source_id.clone());
        }
        groups.entry(hit.source_id.clone()).or_default().push(hit);
    }
    let mut lanes: Vec<std::collections::VecDeque<SourceHit>> = source_order
        .into_iter()
        .map(|sid| groups.remove(&sid).unwrap_or_default().into_iter().take(per_source).collect())
        .collect();
    let mut out = Vec::new();
    loop {
        let mut advanced = false;
        for lane in &mut lanes {
            if let Some(hit) = lane.pop_front() {
                out.push(hit);
                advanced = true;
            }
        }
        if !advanced { break; }
    }
    out
}

fn merge_hits(hits: Vec<SourceHit>) -> Vec<(String, String, crate::indexer::source::MangaResult, Vec<SourceAlternative>)> {
    // preserve insertion order per title — first source wins as primary
    let mut order: Vec<String> = Vec::new();
    let mut groups: std::collections::HashMap<String, (String, String, crate::indexer::source::MangaResult, Vec<SourceAlternative>)> =
        std::collections::HashMap::new();

    for hit in hits {
        let key = normalize_title(&hit.result.title);
        if let Some(entry) = groups.get_mut(&key) {
            entry.3.push(SourceAlternative {
                source: hit.source_id,
                source_name: hit.source_name,
                id: hit.result.id,
                cover_url: hit.result.cover_url,
                status: hit.result.status,
            });
        } else {
            order.push(key.clone());
            groups.insert(key, (hit.source_id, hit.source_name, hit.result, vec![]));
        }
    }

    order.into_iter().filter_map(|k| groups.remove(&k)).collect()
}

async fn check_in_library(
    db: &sqlx::SqlitePool,
    user_id: &str,
    source: &str,
    source_id: &str,
    alternatives: &[SourceAlternative],
) -> (bool, Option<String>) {
    // Check primary
    if let Ok(Some(row)) = sqlx::query!(
        "SELECT m.id FROM manga m JOIN user_manga um ON um.manga_id = m.id AND um.user_id = ? WHERE m.source = ? AND m.source_id = ?",
        user_id, source, source_id
    )
    .fetch_optional(db)
    .await
    {
        return (true, row.id);
    }
    // Check alternatives
    for alt in alternatives {
        if let Ok(Some(row)) = sqlx::query!(
            "SELECT m.id FROM manga m JOIN user_manga um ON um.manga_id = m.id AND um.user_id = ? WHERE m.source = ? AND m.source_id = ?",
            user_id, alt.source, alt.id
        )
        .fetch_optional(db)
        .await
        {
            return (true, row.id);
        }
    }
    (false, None)
}

// ── Handlers ──────────────────────────────────────────────────────────────────

async fn search(
    State(state): State<AppState>,
    axum::Extension(claims): axum::Extension<auth::Claims>,
    Query(q): Query<SearchQuery>,
) -> ApiResult<Response> {
    let ct = q.content_type.as_deref();
    let sources = snapshot_sources(&*state.sources.read().await, ct);
    if sources.is_empty() {
        return Ok(Json(Vec::<SearchResult>::new()).into_response());
    }
    let hits = fan_out(sources, FanOutMode::Search(q.q)).await;
    let merged = merge_hits(hits);
    let user_id = &claims.sub;
    let mut out = Vec::with_capacity(merged.len());

    for (src_id, src_name, r, alts) in merged {
        let (in_library, library_id) = check_in_library(&state.db, user_id, &src_id, &r.id, &alts).await;
        out.push(SearchResult {
            in_library,
            library_id,
            source: src_id,
            source_name: src_name,
            id: r.id,
            title: r.title,
            description: r.description,
            cover_url: r.cover_url,
            status: r.status,
            author: r.author,
            year: r.year,
            tags: r.tags,
            content_type: r.content_type.unwrap_or_else(|| "manga".to_string()),
            alternatives: alts,
        });
    }

    let sources_for_seed: Vec<(String, Arc<dyn Source>)> = {
        let reg = state.sources.read().await;
        reg.iter().map(|(id, src)| (id.clone(), src.clone())).collect()
    };
    let keys: Vec<String> = out.iter().map(|r| normalize_title(&r.title)).collect();
    let meta_cache = batch_lookup_meta(&state.db, &keys).await;
    seed_results_meta(&state.db, &sources_for_seed, &state.config.download_dir, &out, &meta_cache).await;
    enrich_with_cache(&meta_cache, &mut out);
    Ok(Json(out).into_response())
}

async fn trending(
    State(state): State<AppState>,
    axum::Extension(claims): axum::Extension<auth::Claims>,
) -> ApiResult<Response> {
    const CACHE_TTL: std::time::Duration = std::time::Duration::from_secs(3600);

    let per_source: usize = sqlx::query_scalar!(
        "SELECT value FROM server_settings WHERE key = 'trending_per_source'"
    )
    .fetch_optional(&state.db)
    .await
    .ok()
    .flatten()
    .and_then(|v: String| v.parse().ok())
    .unwrap_or(5);

    // Snapshot sources once for seeding (used in all response paths)
    let sources_for_seed: Vec<(String, Arc<dyn Source>)> = {
        let reg = state.sources.read().await;
        reg.iter().map(|(id, src)| (id.clone(), src.clone())).collect()
    };

    // Serve from cache if still fresh
    {
        let guard = state.trending_cache.lock().await;
        if let Some((ts, entries)) = guard.as_ref() {
            if ts.elapsed() < CACHE_TTL {
                return build_trending_response(&state.db, &claims.sub, entries, &sources_for_seed, &state.config.download_dir).await;
            }
        }
    }

    // Fetch fresh results from all sources
    let sources = snapshot_sources(&*state.sources.read().await, None);
    let fresh = if sources.is_empty() {
        vec![]
    } else {
        fan_out(sources, FanOutMode::Trending).await
    };

    // On empty fetch, serve stale cache or empty
    if fresh.is_empty() {
        let guard = state.trending_cache.lock().await;
        if let Some((_, entries)) = guard.as_ref() {
            tracing::debug!("trending: serving stale cache (sources returned nothing)");
            return build_trending_response(&state.db, &claims.sub, entries, &sources_for_seed, &state.config.download_dir).await;
        }
        return Ok(Json(Vec::<SearchResult>::new()).into_response());
    }

    // Interleave, limit per source, merge duplicates, then cache
    let limited = interleave_and_limit(fresh, per_source);
    let merged = merge_hits(limited);
    let entries: Vec<CachedTrendingEntry> = merged
        .into_iter()
        .map(|(source_id, source_name, result, alternatives)| CachedTrendingEntry {
            source_id, source_name, result, alternatives,
        })
        .collect();

    {
        let mut guard = state.trending_cache.lock().await;
        *guard = Some((Instant::now(), entries.clone()));
    }

    build_trending_response(&state.db, &claims.sub, &entries, &sources_for_seed, &state.config.download_dir).await
}

async fn build_trending_response(
    db: &sqlx::SqlitePool,
    user_id: &str,
    entries: &[CachedTrendingEntry],
    sources_for_seed: &[(String, Arc<dyn Source>)],
    download_dir: &str,
) -> ApiResult<Response> {
    let mut out = Vec::with_capacity(entries.len());
    for entry in entries {
        let (in_library, library_id) =
            check_in_library(db, user_id, &entry.source_id, &entry.result.id, &entry.alternatives).await;
        out.push(SearchResult {
            in_library,
            library_id,
            source: entry.source_id.clone(),
            source_name: entry.source_name.clone(),
            id: entry.result.id.clone(),
            title: entry.result.title.clone(),
            description: entry.result.description.clone(),
            cover_url: entry.result.cover_url.clone(),
            status: entry.result.status.clone(),
            author: entry.result.author.clone(),
            year: entry.result.year,
            tags: entry.result.tags.clone(),
            content_type: entry.result.content_type.clone().unwrap_or_else(|| "manga".to_string()),
            alternatives: entry.alternatives.clone(),
        });
    }
    let keys: Vec<String> = out.iter().map(|r| normalize_title(&r.title)).collect();
    let meta_cache = batch_lookup_meta(db, &keys).await;
    seed_results_meta(db, sources_for_seed, download_dir, &out, &meta_cache).await;
    enrich_with_cache(&meta_cache, &mut out);
    Ok(Json(out).into_response())
}

async fn detail_meta(
    State(state): State<AppState>,
    Query(q): Query<DetailQuery>,
) -> ApiResult<Response> {
    let title_key = q.title.as_deref().map(normalize_title);

    // Cache hit → return immediately (re-trigger download if cover missing locally)
    if let Some(ref key) = title_key {
        if let Ok(Some(cached)) = sqlx::query!(
            "SELECT description, cover_local_path, cover_cdn_url, tags, chapter_count FROM title_meta WHERE title_key = ?",
            key
        )
        .fetch_optional(&state.db)
        .await
        {
            if cached.cover_local_path.is_none() {
                if let Some(ref cdn_url) = cached.cover_cdn_url {
                    let src = state.sources.read().await.get(&q.source).cloned();
                    if let Some(src) = src {
                        spawn_cover_download(
                            state.db.clone(), src, cdn_url.clone(),
                            key.clone(), state.config.download_dir.clone(),
                        );
                    }
                }
            }
            let cover_url = if cached.cover_local_path.is_some() {
                Some(meta_cover_url(key))
            } else {
                cached.cover_cdn_url
            };
            return Ok(Json(MangaDetailResult {
                description: cached.description,
                cover_url,
                chapter_count: cached.chapter_count as usize,
                tags: cached.tags,
            }).into_response());
        }
    }

    // Cache miss → fetch from source
    let src = state.sources.read().await.get(&q.source).cloned();
    let Some(src) = src else {
        return Ok(StatusCode::BAD_GATEWAY.into_response());
    };

    let meta = match src.fetch_meta(&q.source_id).await {
        Ok(m) => m,
        Err(e) => {
            tracing::error!("detail fetch error: {}", e);
            return Ok(StatusCode::BAD_GATEWAY.into_response());
        }
    };

    // Store in cache and spawn cover download
    if let Some(ref key) = title_key {
        store_meta(
            &state.db, key, &q.source, &q.source_id,
            meta.description.as_deref(),
            meta.cover_url.as_deref(),
            meta.tags.as_deref(),
            meta.chapter_count,
        ).await;

        if let Some(cdn_url) = meta.cover_url.clone() {
            let db = state.db.clone();
            let dir = state.config.download_dir.clone();
            let key = key.clone();
            tokio::spawn(async move {
                download_meta_cover(&db, src, cdn_url, key, dir).await;
            });
        }
    }

    Ok(Json(MangaDetailResult {
        description: meta.description,
        cover_url: meta.cover_url,
        chapter_count: meta.chapter_count,
        tags: meta.tags,
    }).into_response())
}

async fn add_manga(
    State(state): State<AppState>,
    axum::Extension(claims): axum::Extension<auth::Claims>,
    Json(body): Json<AddMangaRequest>,
) -> ApiResult<Response> {
    let now = Utc::now();
    let source = body.source.as_str();

    let src = state.sources.read().await.get(source).cloned();
    let Some(src) = src else {
        return Ok(StatusCode::BAD_REQUEST.into_response());
    };

    let existing_id = sqlx::query_scalar!(
        r#"SELECT id as "id!" FROM manga WHERE source = ? AND source_id = ?"#,
        source, body.source_id
    )
    .fetch_optional(&state.db)
    .await?;

    let manga_id: String = if let Some(eid) = existing_id {
        sqlx::query!(
            "UPDATE manga SET \
             description = COALESCE(description, ?), \
             author = COALESCE(author, ?), \
             year = COALESCE(year, ?), \
             tags = COALESCE(tags, ?), \
             sync_status = 'syncing' \
             WHERE id = ?",
            body.description, body.author, body.year, body.tags, eid
        )
        .execute(&state.db)
        .await?;
        eid
    } else {
        let id = Uuid::new_v4().to_string();
        let tag_explicit = body.tags.as_deref()
            .map(|t| t.split(',').any(|tag| tag.trim().eq_ignore_ascii_case("adult")))
            .unwrap_or(false);
        let is_explicit: i64 = if src.default_explicit() || tag_explicit { 1 } else { 0 };
        sqlx::query!(
            r#"INSERT INTO manga
               (id, title, description, cover_url, status, source, source_id,
                author, year, tags, sync_status, content_type, is_explicit, created_at, updated_at)
               VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, 'syncing', ?, ?, ?, ?)"#,
            id, body.title, body.description, body.cover_url, body.status,
            source, body.source_id, body.author, body.year, body.tags,
            body.content_type, is_explicit, now, now
        )
        .execute(&state.db)
        .await?;
        id
    };

    let now_str = now.to_rfc3339();
    sqlx::query!(
        "INSERT OR IGNORE INTO user_manga (user_id, manga_id, added_at) VALUES (?, ?, ?)",
        claims.sub, manga_id, now_str
    )
    .execute(&state.db)
    .await?;

    let db = state.db.clone();
    let mid = manga_id.clone();
    let source_id = body.source_id.clone();
    let cover_url = body.cover_url.clone();
    let download_dir = state.config.download_dir.clone();
    tokio::spawn(async move {
        let result = src.sync_chapters(&db, &mid, &source_id).await;
        if let Err(e) = &result {
            tracing::error!("chapter sync failed for {}: {}", mid, e);
        }
        let status = if result.is_ok() { "ready" } else { "error" };
        let _ = sqlx::query!("UPDATE manga SET sync_status = ? WHERE id = ?", status, mid)
            .execute(&db)
            .await;

        if let Some(cdn_url) = cover_url {
            let ext = cdn_url.split('?').next().unwrap_or("cover")
                .rsplit('.').next().unwrap_or("jpg");
            let cover_path = Path::new(&download_dir)
                .join("_covers")
                .join(format!("{}.{}", mid, ext));
            match src.fetch_cover(&cdn_url).await {
                Ok(bytes) => {
                    if let Some(parent) = cover_path.parent() {
                        let _ = tokio::fs::create_dir_all(parent).await;
                    }
                    if tokio::fs::write(&cover_path, bytes).await.is_ok() {
                        let path_str = cover_path.to_string_lossy().to_string();
                        let _ = sqlx::query!(
                            "UPDATE manga SET cover_url = ? WHERE id = ?",
                            path_str, mid
                        )
                        .execute(&db)
                        .await;
                    }
                }
                Err(e) => tracing::warn!("cover download failed for {}: {}", mid, e),
            }
        }
    });

    let manga = fetch_manga(&state.db, &manga_id).await?;
    Ok(Json(manga).into_response())
}

pub async fn fetch_manga(db: &sqlx::SqlitePool, id: &str) -> Result<Manga, sqlx::Error> {
    sqlx::query_as!(
        Manga,
        r#"SELECT
               id as "id!",
               title as "title!",
               description,
               cover_url,
               status as "status!",
               source as "source!",
               source_id,
               local_path,
               author,
               year,
               tags,
               sync_status as "sync_status!",
               content_type as "content_type!",
               (is_explicit != 0) as "is_explicit!: bool",
               created_at as "created_at: _",
               updated_at as "updated_at: _"
           FROM manga WHERE id = ?"#,
        id
    )
    .fetch_one(db)
    .await
}
