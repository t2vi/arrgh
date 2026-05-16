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
use uuid::Uuid;

use crate::{db::Manga, AppState};
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
}

#[derive(Deserialize)]
pub struct SearchQuery {
    pub q: String,
}

#[derive(Serialize)]
pub struct SearchResult {
    pub id: String,
    pub source: String,
    pub title: String,
    pub description: Option<String>,
    pub cover_url: Option<String>,
    pub status: String,
    pub author: Option<String>,
    pub year: Option<i64>,
    pub tags: Option<String>,
    pub in_library: bool,
    pub library_id: Option<String>,
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

async fn search(
    State(state): State<AppState>,
    Query(q): Query<SearchQuery>,
) -> ApiResult<Response> {
    let source_id = "mangapill";
    let Some(src) = state.sources.get(source_id) else {
        return Ok(StatusCode::BAD_GATEWAY.into_response());
    };

    let results = match src.search(&q.q).await {
        Ok(r) => r,
        Err(e) => {
            tracing::error!("{} search error: {}", source_id, e);
            return Ok(StatusCode::BAD_GATEWAY.into_response());
        }
    };

    let mut out = Vec::with_capacity(results.len());
    for r in results {
        let row = sqlx::query!(
            "SELECT id FROM manga WHERE source = ? AND source_id = ?",
            source_id, r.id
        )
        .fetch_optional(&state.db)
        .await?;

        out.push(SearchResult {
            in_library: row.is_some(),
            library_id: row.and_then(|r| r.id),
            source: source_id.to_string(),
            id: r.id,
            title: r.title,
            description: r.description,
            cover_url: r.cover_url,
            status: r.status,
            author: r.author,
            year: r.year,
            tags: r.tags,
        });
    }

    Ok(Json(out).into_response())
}

async fn detail_meta(
    State(state): State<AppState>,
    Query(q): Query<DetailQuery>,
) -> ApiResult<Response> {
    let Some(src) = state.sources.get(&q.source) else {
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
    }).into_response())
}

async fn trending(
    State(state): State<AppState>,
) -> ApiResult<Response> {
    let source_id = "mangapill";
    let Some(src) = state.sources.get(source_id) else {
        return Ok(StatusCode::BAD_GATEWAY.into_response());
    };

    let results = match src.trending().await {
        Ok(r) => r,
        Err(e) => {
            tracing::error!("trending fetch error: {}", e);
            return Ok(StatusCode::BAD_GATEWAY.into_response());
        }
    };

    let mut out = Vec::with_capacity(results.len());
    for r in results {
        let row = sqlx::query!(
            "SELECT id FROM manga WHERE source = ? AND source_id = ?",
            source_id, r.id
        )
        .fetch_optional(&state.db)
        .await?;

        out.push(SearchResult {
            in_library: row.is_some(),
            library_id: row.and_then(|r| r.id),
            source: source_id.to_string(),
            id: r.id,
            title: r.title,
            description: r.description,
            cover_url: r.cover_url,
            status: r.status,
            author: r.author,
            year: r.year,
            tags: r.tags,
        });
    }

    Ok(Json(out).into_response())
}

async fn add_manga(
    State(state): State<AppState>,
    Json(body): Json<AddMangaRequest>,
) -> ApiResult<Response> {
    let now = Utc::now();
    let source = body.source.as_str();

    let Some(src) = state.sources.get(source).cloned() else {
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
        let is_explicit: i64 = if src.default_explicit() { 1 } else { 0 };
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
        let _ = sqlx::query!(
            "UPDATE manga SET sync_status = ? WHERE id = ?",
            status, mid
        )
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
