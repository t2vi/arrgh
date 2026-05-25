use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::{IntoResponse, Json, Response},
    routing::{get, post},
    Router,
};
use chrono::Utc;
use serde_json::json;
use uuid::Uuid;

use crate::{auth, db::Chapter, indexer::local::verify_manga_downloads, AppState};
use super::ApiResult;

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/chapters/manga/{manga_id}", get(list_chapters))
        .route("/chapters/{id}", get(get_chapter))
        .route("/chapters/{id}/text", get(get_chapter_text))
        .route("/chapters/{id}/download", post(queue_download))
}

async fn list_chapters(
    State(state): State<AppState>,
    axum::Extension(claims): axum::Extension<auth::Claims>,
    Path(manga_id): Path<String>,
) -> ApiResult<Json<Vec<Chapter>>> {
    let allow_explicit: i64 = if claims.allow_explicit { 1 } else { 0 };

    let chapters = sqlx::query_as!(
        Chapter,
        r#"SELECT
               c.id as "id!",
               c.manga_id as "manga_id!",
               c.title,
               c.number,
               c.volume,
               c.local_path,
               c.page_count,
               (c.downloaded != 0) as "downloaded!: bool",
               (EXISTS(SELECT 1 FROM chapter_sources cs WHERE cs.chapter_id = c.id)) as "has_sources!: bool",
               c.chapter_format as "chapter_format!",
               c.created_at as "created_at: _"
           FROM chapters c
           JOIN manga m ON c.manga_id = m.id
           WHERE c.manga_id = ? AND (m.is_explicit = 0 OR ? = 1)
           ORDER BY c.number ASC"#,
        manga_id, allow_explicit
    )
    .fetch_all(&state.db)
    .await?;

    // Background: reset any stale downloaded=1 entries whose files are gone
    let db = state.db.clone();
    let mid = manga_id.clone();
    tokio::spawn(async move {
        if let Err(e) = verify_manga_downloads(&db, &mid).await {
            tracing::warn!("verify_manga_downloads error for {}: {}", mid, e);
        }
    });

    Ok(Json(chapters))
}

async fn queue_download(
    State(state): State<AppState>,
    axum::Extension(claims): axum::Extension<auth::Claims>,
    Path(id): Path<String>,
) -> ApiResult<StatusCode> {
    let allow_explicit: i64 = if claims.allow_explicit { 1 } else { 0 };
    let row = sqlx::query!(
        r#"SELECT c.id, c.number, m.title as manga_title
           FROM chapters c JOIN manga m ON c.manga_id = m.id
           WHERE c.id = ? AND c.downloaded = 0
             AND EXISTS(SELECT 1 FROM chapter_sources cs WHERE cs.chapter_id = c.id)
             AND (m.is_explicit = 0 OR ? = 1)"#,
        id, allow_explicit
    )
    .fetch_optional(&state.db)
    .await?;

    let Some(row) = row else {
        return Ok(StatusCode::NOT_FOUND);
    };

    let now = Utc::now();
    let queue_id = Uuid::new_v4().to_string();

    sqlx::query!(
        r#"INSERT INTO download_queue (id, chapter_id, manga_title, chapter_num, status, created_at, updated_at, queued_by)
           VALUES (?, ?, ?, ?, 'pending', ?, ?, ?)
           ON CONFLICT(chapter_id) DO UPDATE SET
             status = 'pending',
             error = NULL,
             queued_by = excluded.queued_by,
             updated_at = excluded.updated_at
           WHERE download_queue.status IN ('error', 'cancelled')"#,
        queue_id, row.id, row.manga_title, row.number, now, now, claims.sub
    )
    .execute(&state.db)
    .await?;

    Ok(StatusCode::ACCEPTED)
}

async fn get_chapter(
    State(state): State<AppState>,
    axum::Extension(claims): axum::Extension<auth::Claims>,
    Path(id): Path<String>,
) -> ApiResult<Response> {
    let allow_explicit: i64 = if claims.allow_explicit { 1 } else { 0 };
    let chapter = sqlx::query_as!(
        Chapter,
        r#"SELECT
               c.id as "id!",
               c.manga_id as "manga_id!",
               c.title,
               c.number,
               c.volume,
               c.local_path,
               c.page_count,
               (c.downloaded != 0) as "downloaded!: bool",
               (EXISTS(SELECT 1 FROM chapter_sources cs WHERE cs.chapter_id = c.id)) as "has_sources!: bool",
               c.chapter_format as "chapter_format!",
               c.created_at as "created_at: _"
           FROM chapters c
           JOIN manga m ON c.manga_id = m.id
           WHERE c.id = ? AND (m.is_explicit = 0 OR ? = 1)"#,
        id, allow_explicit
    )
    .fetch_optional(&state.db)
    .await?;

    let Some(chapter) = chapter else {
        return Ok(StatusCode::NOT_FOUND.into_response());
    };
    Ok(Json(chapter).into_response())
}

async fn get_chapter_text(
    State(state): State<AppState>,
    axum::Extension(claims): axum::Extension<auth::Claims>,
    Path(id): Path<String>,
) -> ApiResult<Response> {
    let allow_explicit: i64 = if claims.allow_explicit { 1 } else { 0 };
    let row = sqlx::query!(
        r#"SELECT c.local_path, c.downloaded, c.chapter_format as "chapter_format!"
           FROM chapters c JOIN manga m ON c.manga_id = m.id
           WHERE c.id = ? AND (m.is_explicit = 0 OR ? = 1)"#,
        id, allow_explicit
    )
    .fetch_optional(&state.db)
    .await?;

    let Some(row) = row else {
        return Ok(StatusCode::NOT_FOUND.into_response());
    };
    if row.chapter_format != "text" {
        return Ok(StatusCode::BAD_REQUEST.into_response());
    }
    if row.downloaded == 0 {
        return Ok(StatusCode::NOT_FOUND.into_response());
    }
    let path = match row.local_path {
        Some(p) => p,
        None => return Ok(StatusCode::NOT_FOUND.into_response()),
    };
    let content = match tokio::fs::read_to_string(&path).await {
        Ok(c) => c,
        Err(_) => return Ok(StatusCode::NOT_FOUND.into_response()),
    };
    Ok(Json(json!({ "content": content })).into_response())
}
