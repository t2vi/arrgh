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
use uuid::Uuid;

use crate::{auth, db::Manga, indexer::source::Source, AppState};
use super::ApiResult;

#[derive(Deserialize)]
pub struct DetailQuery {
    pub source: String,
    pub source_id: String,
}

#[derive(Serialize)]
pub struct MangaDetailResult {
    pub description: Option<String>,
    pub cover_url: Option<String>,
    pub chapter_count: usize,
    pub tags: Option<String>,
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
                        tracing::warn!("{} {} failed: {}", sid, mode.label(), e);
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
    if hits.is_empty() {
        return Ok(StatusCode::BAD_GATEWAY.into_response());
    }

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
            alternatives: alts,
        });
    }

    Ok(Json(out).into_response())
}

async fn trending(
    State(state): State<AppState>,
    axum::Extension(claims): axum::Extension<auth::Claims>,
) -> ApiResult<Response> {
    let sources = snapshot_sources(&*state.sources.read().await, None);
    if sources.is_empty() {
        return Ok(Json(Vec::<SearchResult>::new()).into_response());
    }
    let hits = fan_out(sources, FanOutMode::Trending).await;
    if hits.is_empty() {
        return Ok(StatusCode::BAD_GATEWAY.into_response());
    }

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
            alternatives: alts,
        });
    }

    Ok(Json(out).into_response())
}

async fn detail_meta(
    State(state): State<AppState>,
    Query(q): Query<DetailQuery>,
) -> ApiResult<Response> {
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
    let manga_dir = state.config.manga_dir.clone();
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
            let cover_path = Path::new(&manga_dir)
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
