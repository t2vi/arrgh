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

use crate::{auth, db::Manga, mangaupdates::{MangaUpdatesClient, MuSeries}, AppState};
use super::ApiResult;

// ── Trending cache ────────────────────────────────────────────────────────────

pub type TrendingCache = Arc<Mutex<Option<(Instant, Vec<MuSeries>)>>>;

// ── Wire types ────────────────────────────────────────────────────────────────

#[derive(Deserialize)]
pub struct SearchQuery {
    pub q: String,
}

#[derive(Serialize)]
pub struct SearchResult {
    pub mangaupdates_id: String,
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
}

#[derive(Deserialize)]
pub struct AddMangaRequest {
    pub mangaupdates_id: String,
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
        .route("/discover/add", post(add_manga))
}

// ── Helpers ───────────────────────────────────────────────────────────────────

pub fn normalize_title(title: &str) -> String {
    title
        .chars()
        .map(|c| if c.is_alphanumeric() { c } else { ' ' })
        .collect::<String>()
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .to_lowercase()
}

fn meta_cover_url(key: &str) -> String {
    format!("/api/media/meta-cover?key={}", urlencoding::encode(key))
}

async fn check_in_library(
    db: &sqlx::SqlitePool,
    user_id: &str,
    mangaupdates_id: &str,
) -> (bool, Option<String>) {
    match sqlx::query!(
        r#"SELECT m.id as "id!" FROM manga m
           JOIN user_manga um ON um.manga_id = m.id AND um.user_id = ?
           WHERE m.mangaupdates_id = ?"#,
        user_id, mangaupdates_id
    )
    .fetch_optional(db)
    .await
    {
        Ok(Some(row)) => (true, Some(row.id)),
        _ => (false, None),
    }
}

async fn enrich_cover(
    db: &sqlx::SqlitePool,
    result: &mut SearchResult,
) {
    let key = normalize_title(&result.title);
    match sqlx::query!(
        "SELECT cover_local_path, cover_cdn_url FROM title_meta WHERE title_key = ?",
        key
    )
    .fetch_optional(db)
    .await
    {
        Ok(Some(row)) => {
            if row.cover_local_path.is_some() {
                result.cover_url = Some(meta_cover_url(&key));
            } else if result.cover_url.is_none() {
                result.cover_url = row.cover_cdn_url;
            }
        }
        _ => {}
    }
}

async fn seed_and_cache_cover(
    db: sqlx::SqlitePool,
    http: reqwest::Client,
    title: String,
    cdn_url: String,
    download_dir: String,
) {
    let key = normalize_title(&title);
    let now = Utc::now().to_rfc3339();

    // Upsert title_meta row
    if let Err(e) = sqlx::query!(
        r#"INSERT INTO title_meta (title_key, cover_cdn_url, fetched_at)
           VALUES (?, ?, ?)
           ON CONFLICT(title_key) DO UPDATE SET
             cover_cdn_url = COALESCE(title_meta.cover_cdn_url, excluded.cover_cdn_url),
             fetched_at    = excluded.fetched_at"#,
        key, cdn_url, now
    )
    .execute(&db)
    .await
    {
        tracing::warn!("seed_and_cache_cover insert error for '{}': {}", key, e);
        return;
    }

    // Download cover bytes directly via reqwest (MU CDN, no plugin needed)
    let bytes = match http
        .get(&cdn_url)
        .header("User-Agent", "Mozilla/5.0")
        .send()
        .await
        .and_then(|r| r.error_for_status())
    {
        Ok(resp) => match resp.bytes().await {
            Ok(b) => b,
            Err(e) => { tracing::warn!("cover read error for '{}': {}", key, e); return; }
        },
        Err(e) => { tracing::warn!("cover fetch error for '{}': {}", key, e); return; }
    };

    let ext = cdn_url.split('?').next().unwrap_or("cover").rsplit('.').next().unwrap_or("jpg");
    let safe = key.chars().map(|c| if c.is_alphanumeric() { c } else { '_' }).collect::<String>();
    let path = Path::new(&download_dir).join("_meta").join(format!("{}.{}", safe, ext));

    if let Some(parent) = path.parent() {
        let _ = tokio::fs::create_dir_all(parent).await;
    }
    if tokio::fs::write(&path, &bytes).await.is_ok() {
        let path_str = path.to_string_lossy().to_string();
        let _ = sqlx::query!(
            "UPDATE title_meta SET cover_local_path = ? WHERE title_key = ?",
            path_str, key
        )
        .execute(&db)
        .await;
    }
}

fn mu_to_search_result(s: &MuSeries, in_library: bool, library_id: Option<String>) -> SearchResult {
    SearchResult {
        mangaupdates_id: s.series_id.to_string(),
        title: s.title.clone(),
        description: s.description.clone(),
        cover_url: s.cover_url.clone(),
        status: s.status.clone(),
        author: s.author.clone(),
        year: s.year,
        tags: s.tags.clone(),
        content_type: s.content_type.clone(),
        in_library,
        library_id,
    }
}

// ── Handlers ──────────────────────────────────────────────────────────────────

async fn search(
    State(state): State<AppState>,
    axum::Extension(claims): axum::Extension<auth::Claims>,
    Query(q): Query<SearchQuery>,
) -> ApiResult<Response> {
    let mu = MangaUpdatesClient::new(&state.http);
    let series = match mu.search(&q.q, 1).await {
        Ok(s) => s,
        Err(e) => {
            tracing::error!("MangaUpdates search error: {}", e);
            return Ok(StatusCode::BAD_GATEWAY.into_response());
        }
    };

    let mut results = Vec::with_capacity(series.len());
    for s in &series {
        let mu_id = s.series_id.to_string();
        let (in_library, library_id) = check_in_library(&state.db, &claims.sub, &mu_id).await;
        let mut r = mu_to_search_result(s, in_library, library_id);
        enrich_cover(&state.db, &mut r).await;

        // Seed cover in background if we have a CDN URL
        if let Some(ref cdn) = s.cover_url {
            tokio::spawn(seed_and_cache_cover(
                state.db.clone(),
                state.http.clone(),
                s.title.clone(),
                cdn.clone(),
                state.config.download_dir.clone(),
            ));
        }

        results.push(r);
    }

    Ok(Json(results).into_response())
}

async fn trending(
    State(state): State<AppState>,
    axum::Extension(claims): axum::Extension<auth::Claims>,
) -> ApiResult<Response> {
    const CACHE_TTL: std::time::Duration = std::time::Duration::from_secs(3600);

    let cached = {
        let guard = state.trending_cache.lock().await;
        guard.as_ref().and_then(|(ts, entries)| {
            if ts.elapsed() < CACHE_TTL { Some(entries.clone()) } else { None }
        })
    };

    let series = if let Some(c) = cached {
        c
    } else {
        let mu = MangaUpdatesClient::new(&state.http);
        let fresh = match mu.latest_releases().await {
            Ok(s) if !s.is_empty() => s,
            Ok(_) => {
                // Empty result — serve stale if available
                let guard = state.trending_cache.lock().await;
                if let Some((_, entries)) = guard.as_ref() {
                    tracing::debug!("trending: serving stale cache (MU returned nothing)");
                    entries.clone()
                } else {
                    return Ok(Json(Vec::<SearchResult>::new()).into_response());
                }
            }
            Err(e) => {
                tracing::error!("MangaUpdates latest_releases error: {}", e);
                // Serve stale if available
                let guard = state.trending_cache.lock().await;
                if let Some((_, entries)) = guard.as_ref() {
                    entries.clone()
                } else {
                    return Ok(StatusCode::BAD_GATEWAY.into_response());
                }
            }
        };
        {
            let mut guard = state.trending_cache.lock().await;
            *guard = Some((Instant::now(), fresh.clone()));
        }
        fresh
    };

    let mut results = Vec::with_capacity(series.len());
    for s in &series {
        let mu_id = s.series_id.to_string();
        let (in_library, library_id) = check_in_library(&state.db, &claims.sub, &mu_id).await;
        let mut r = mu_to_search_result(s, in_library, library_id);
        enrich_cover(&state.db, &mut r).await;
        results.push(r);
    }

    Ok(Json(results).into_response())
}

async fn add_manga(
    State(state): State<AppState>,
    axum::Extension(claims): axum::Extension<auth::Claims>,
    Json(body): Json<AddMangaRequest>,
) -> ApiResult<Response> {
    let now = Utc::now();
    let now_str = now.to_rfc3339();

    // Resolve cover: if the client passed a meta-cover path, resolve back to CDN URL
    let resolved_cover_url: Option<String> = if let Some(ref url) = body.cover_url {
        if let Some(encoded_key) = url.strip_prefix("/api/media/meta-cover?key=") {
            let key = urlencoding::decode(encoded_key)
                .unwrap_or_else(|_| encoded_key.into())
                .into_owned();
            sqlx::query_scalar!(
                "SELECT cover_cdn_url FROM title_meta WHERE title_key = ?",
                key
            )
            .fetch_optional(&state.db)
            .await?
            .flatten()
        } else {
            Some(url.clone())
        }
    } else {
        None
    };

    // Check if manga with this MU ID already exists
    let existing: Option<String> = sqlx::query_scalar!(
        r#"SELECT id as "id!" FROM manga WHERE mangaupdates_id = ?"#,
        body.mangaupdates_id
    )
    .fetch_optional(&state.db)
    .await?;

    let manga_id: String = if let Some(eid) = existing {
        // Already in catalog — update metadata if missing
        sqlx::query!(
            "UPDATE manga SET \
             description  = COALESCE(description, ?), \
             author       = COALESCE(author, ?), \
             year         = COALESCE(year, ?), \
             tags         = COALESCE(tags, ?) \
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
        let is_explicit: i64 = if tag_explicit { 1 } else { 0 };

        sqlx::query!(
            r#"INSERT INTO manga
               (id, mangaupdates_id, title, description, cover_url, status,
                author, year, tags, sync_status, content_type, is_explicit, created_at, updated_at)
               VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, 'syncing', ?, ?, ?, ?)"#,
            id, body.mangaupdates_id, body.title, body.description, resolved_cover_url,
            body.status, body.author, body.year, body.tags,
            body.content_type, is_explicit, now, now
        )
        .execute(&state.db)
        .await?;
        id
    };

    // Subscribe this user to the manga
    sqlx::query!(
        "INSERT OR IGNORE INTO user_manga (user_id, manga_id, added_at) VALUES (?, ?, ?)",
        claims.sub, manga_id, now_str
    )
    .execute(&state.db)
    .await?;

    // Async: download cover + match sources + sync chapters
    let db = state.db.clone();
    let http = state.http.clone();
    let sources = state.sources.clone();
    let mid = manga_id.clone();
    let title = body.title.clone();
    let cover_cdn = resolved_cover_url.clone();
    let download_dir = state.config.download_dir.clone();

    tokio::spawn(async move {
        // 1. Download manga cover
        if let Some(cdn_url) = cover_cdn {
            let ext = cdn_url.split('?').next().unwrap_or("cover")
                .rsplit('.').next().unwrap_or("jpg");
            let cover_path = Path::new(&download_dir)
                .join("_covers")
                .join(format!("{}.{}", mid, ext));
            match http.get(&cdn_url)
                .header("User-Agent", "Mozilla/5.0")
                .send()
                .await
                .and_then(|r| r.error_for_status())
            {
                Ok(resp) => match resp.bytes().await {
                    Ok(bytes) => {
                        if let Some(parent) = cover_path.parent() {
                            let _ = tokio::fs::create_dir_all(parent).await;
                        }
                        if tokio::fs::write(&cover_path, &bytes).await.is_ok() {
                            let path_str = cover_path.to_string_lossy().to_string();
                            let _ = sqlx::query!(
                                "UPDATE manga SET cover_url = ? WHERE id = ?",
                                path_str, mid
                            )
                            .execute(&db)
                            .await;
                        }
                    }
                    Err(e) => tracing::warn!("cover read failed for {}: {}", mid, e),
                },
                Err(e) => tracing::warn!("cover fetch failed for {}: {}", mid, e),
            }
        }

        // 2. Match against registered sources by title search
        let norm = normalize_title(&title);
        let source_snapshot: Vec<(String, Arc<dyn crate::indexer::Source>)> = {
            let reg = sources.read().await;
            reg.iter().map(|(id, src)| (id.clone(), src.clone())).collect()
        };

        let mut any_ok = false;
        for (src_key, src) in &source_snapshot {
            let results = match src.search(&title).await {
                Ok(r) => r,
                Err(e) => {
                    tracing::debug!("source match search '{}' for '{}': {}", src_key, title, e);
                    continue;
                }
            };
            let best = results.iter().find(|r| normalize_title(&r.title) == norm);
            let Some(hit) = best else { continue };

            let ms_id = Uuid::new_v4().to_string();
            let disc = Utc::now().to_rfc3339();
            if let Err(e) = sqlx::query!(
                "INSERT OR IGNORE INTO manga_sources (id, manga_id, source, source_id, discovered_at) VALUES (?, ?, ?, ?, ?)",
                ms_id, mid, src_key, hit.id, disc
            )
            .execute(&db)
            .await
            {
                tracing::warn!("manga_sources insert error: {}", e);
                continue;
            }

            match src.sync_chapters(&db, &mid, &hit.id).await {
                Ok(_) => { any_ok = true; }
                Err(e) => tracing::error!("chapter sync failed for {} ({}): {}", mid, src_key, e),
            }
        }

        let status = if source_snapshot.is_empty() || any_ok { "ready" } else { "error" };
        let _ = sqlx::query!("UPDATE manga SET sync_status = ? WHERE id = ?", status, mid)
            .execute(&db)
            .await;
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalize_lowercases() {
        assert_eq!(normalize_title("My Hero Academia"), "my hero academia");
    }

    #[test]
    fn normalize_strips_punctuation() {
        assert_eq!(normalize_title("One-Piece!"), "one piece");
    }

    #[test]
    fn normalize_collapses_whitespace() {
        assert_eq!(normalize_title("Solo  Leveling"), "solo leveling");
    }

    #[test]
    fn normalize_empty() {
        assert_eq!(normalize_title(""), "");
    }
}
