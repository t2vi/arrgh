use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::{IntoResponse, Json, Response},
    routing::{get, post},
    Router,
};
use chrono::Utc;
use uuid::Uuid;

use crate::{db::Chapter, AppState};
use super::ApiResult;

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/chapters/manga/{manga_id}", get(list_chapters))
        .route("/chapters/{id}", get(get_chapter))
        .route("/chapters/{id}/download", post(queue_download))
}

async fn list_chapters(
    State(state): State<AppState>,
    Path(manga_id): Path<String>,
) -> ApiResult<Json<Vec<Chapter>>> {
    let chapters = sqlx::query_as!(
        Chapter,
        r#"SELECT
               id as "id!",
               manga_id as "manga_id!",
               title,
               number,
               volume,
               source_id,
               local_path,
               page_count,
               (downloaded != 0) as "downloaded!: bool",
               created_at as "created_at: _"
           FROM chapters WHERE manga_id = ? ORDER BY number ASC"#,
        manga_id
    )
    .fetch_all(&state.db)
    .await?;

    Ok(Json(chapters))
}

async fn queue_download(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> ApiResult<StatusCode> {
    let row = sqlx::query!(
        r#"SELECT c.id, c.number, m.title as manga_title
           FROM chapters c JOIN manga m ON c.manga_id = m.id
           WHERE c.id = ? AND c.source_id IS NOT NULL AND c.downloaded = 0"#,
        id
    )
    .fetch_optional(&state.db)
    .await?;

    let Some(row) = row else {
        return Ok(StatusCode::NOT_FOUND);
    };

    let now = Utc::now();
    let queue_id = Uuid::new_v4().to_string();

    sqlx::query!(
        r#"INSERT INTO download_queue (id, chapter_id, manga_title, chapter_num, status, created_at, updated_at)
           VALUES (?, ?, ?, ?, 'pending', ?, ?)
           ON CONFLICT(chapter_id) DO UPDATE SET
             status = 'pending',
             error = NULL,
             updated_at = excluded.updated_at
           WHERE download_queue.status IN ('error', 'cancelled')"#,
        queue_id, row.id, row.manga_title, row.number, now, now
    )
    .execute(&state.db)
    .await?;

    Ok(StatusCode::ACCEPTED)
}

async fn get_chapter(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> ApiResult<Response> {
    let chapter = sqlx::query_as!(
        Chapter,
        r#"SELECT
               id as "id!",
               manga_id as "manga_id!",
               title,
               number,
               volume,
               source_id,
               local_path,
               page_count,
               (downloaded != 0) as "downloaded!: bool",
               created_at as "created_at: _"
           FROM chapters WHERE id = ?"#,
        id
    )
    .fetch_optional(&state.db)
    .await?;

    let Some(chapter) = chapter else {
        return Ok(StatusCode::NOT_FOUND.into_response());
    };
    Ok(Json(chapter).into_response())
}
