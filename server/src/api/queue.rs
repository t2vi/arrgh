use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::Json,
    routing::{delete, get},
    Router,
};
use chrono::Utc;
use serde::Serialize;

use crate::AppState;
use super::ApiResult;

#[derive(Serialize, sqlx::FromRow)]
pub struct QueueItem {
    pub id: String,
    pub chapter_id: String,
    pub manga_title: String,
    pub chapter_num: f64,
    pub status: String,
    pub error: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/queue", get(list_queue))
        .route("/queue/completed", delete(clear_completed))
        .route("/queue/manga/{manga_id}", get(list_manga_queue))
        .route("/queue/{id}", delete(remove_from_queue))
}

async fn list_queue(
    State(state): State<AppState>,
) -> ApiResult<Json<Vec<QueueItem>>> {
    let items = sqlx::query_as!(
        QueueItem,
        r#"SELECT id as "id!", chapter_id as "chapter_id!", manga_title as "manga_title!",
                  chapter_num as "chapter_num!", status as "status!",
                  error, created_at as "created_at!", updated_at as "updated_at!"
           FROM download_queue ORDER BY created_at DESC LIMIT 100"#
    )
    .fetch_all(&state.db)
    .await?;

    Ok(Json(items))
}

async fn list_manga_queue(
    State(state): State<AppState>,
    Path(manga_id): Path<String>,
) -> ApiResult<Json<Vec<QueueItem>>> {
    let items = sqlx::query_as!(
        QueueItem,
        r#"SELECT dq.id as "id!", dq.chapter_id as "chapter_id!",
                  dq.manga_title as "manga_title!", dq.chapter_num as "chapter_num!",
                  dq.status as "status!", dq.error,
                  dq.created_at as "created_at!", dq.updated_at as "updated_at!"
           FROM download_queue dq
           JOIN chapters c ON c.id = dq.chapter_id
           WHERE c.manga_id = ?
           ORDER BY dq.chapter_num ASC"#,
        manga_id
    )
    .fetch_all(&state.db)
    .await?;

    Ok(Json(items))
}

async fn clear_completed(
    State(state): State<AppState>,
) -> ApiResult<StatusCode> {
    sqlx::query!(
        "DELETE FROM download_queue WHERE status IN ('done', 'cancelled', 'error')"
    )
    .execute(&state.db)
    .await?;
    Ok(StatusCode::NO_CONTENT)
}

async fn remove_from_queue(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> ApiResult<StatusCode> {
    let now = Utc::now();

    let rows = sqlx::query!(
        "DELETE FROM download_queue WHERE id = ? AND status IN ('pending', 'error', 'done', 'cancelled')",
        id
    )
    .execute(&state.db)
    .await?;

    if rows.rows_affected() == 0 {
        sqlx::query!(
            "UPDATE download_queue SET status = 'cancelled', updated_at = ? WHERE id = ?",
            now, id
        )
        .execute(&state.db)
        .await?;
    }

    Ok(StatusCode::NO_CONTENT)
}
