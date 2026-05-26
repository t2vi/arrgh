use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::Json,
    routing::{delete, get},
    Router,
};
use chrono::Utc;
use serde::Serialize;

use crate::{auth, AppState};
use super::ApiResult;

#[derive(Serialize, sqlx::FromRow)]
pub struct QueueItem {
    pub id: String,
    pub chapter_id: String,
    pub manga_title: String,
    pub chapter_num: f64,
    pub status: String,
    pub error: Option<String>,
    pub pages_downloaded: i64,
    pub pages_total: i64,
    pub created_at: String,
    pub updated_at: String,
}

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/queue", get(list_queue))
        .route("/queue/completed", delete(clear_completed))
        .route("/queue/title/{title_id}", get(list_title_queue))
        .route("/queue/{id}", delete(remove_from_queue))
}

async fn list_queue(
    State(state): State<AppState>,
    axum::Extension(claims): axum::Extension<auth::Claims>,
) -> ApiResult<Json<Vec<QueueItem>>> {
    let allow_explicit: i64 = if claims.allow_explicit || claims.role == "admin" { 1 } else { 0 };
    let items = sqlx::query_as!(
        QueueItem,
        r#"SELECT dq.id as "id!", dq.chapter_id as "chapter_id!", dq.manga_title as "manga_title!",
                  dq.chapter_num as "chapter_num!", dq.status as "status!", dq.error,
                  dq.pages_downloaded as "pages_downloaded!", dq.pages_total as "pages_total!",
                  dq.created_at as "created_at!", dq.updated_at as "updated_at!"
           FROM download_queue dq
           JOIN chapters c ON c.id = dq.chapter_id
           JOIN titles m ON m.id = c.title_id
           WHERE (m.is_explicit = 0 OR ? = 1)
           ORDER BY dq.created_at DESC LIMIT 100"#,
        allow_explicit
    )
    .fetch_all(&state.db)
    .await?;

    Ok(Json(items))
}

async fn list_title_queue(
    State(state): State<AppState>,
    axum::Extension(claims): axum::Extension<auth::Claims>,
    Path(title_id): Path<String>,
) -> ApiResult<Json<Vec<QueueItem>>> {
    let allow_explicit: i64 = if claims.allow_explicit || claims.role == "admin" { 1 } else { 0 };
    let items = sqlx::query_as!(
        QueueItem,
        r#"SELECT dq.id as "id!", dq.chapter_id as "chapter_id!",
                  dq.manga_title as "manga_title!", dq.chapter_num as "chapter_num!",
                  dq.status as "status!", dq.error,
                  dq.pages_downloaded as "pages_downloaded!", dq.pages_total as "pages_total!",
                  dq.created_at as "created_at!", dq.updated_at as "updated_at!"
           FROM download_queue dq
           JOIN chapters c ON c.id = dq.chapter_id
           JOIN titles m ON m.id = c.title_id
           WHERE c.title_id = ? AND (m.is_explicit = 0 OR ? = 1)
           ORDER BY dq.chapter_num ASC"#,
        title_id, allow_explicit
    )
    .fetch_all(&state.db)
    .await?;

    Ok(Json(items))
}

async fn clear_completed(
    State(state): State<AppState>,
    axum::Extension(claims): axum::Extension<auth::Claims>,
) -> ApiResult<StatusCode> {
    if claims.role != "admin" {
        return Ok(StatusCode::FORBIDDEN);
    }
    sqlx::query!(
        "DELETE FROM download_queue WHERE status IN ('done', 'cancelled', 'error')"
    )
    .execute(&state.db)
    .await?;
    Ok(StatusCode::NO_CONTENT)
}

async fn remove_from_queue(
    State(state): State<AppState>,
    axum::Extension(claims): axum::Extension<auth::Claims>,
    Path(id): Path<String>,
) -> ApiResult<StatusCode> {
    let item = sqlx::query!(
        r#"SELECT queued_by FROM download_queue WHERE id = ?"#,
        id
    )
    .fetch_optional(&state.db)
    .await?;

    let Some(item) = item else {
        return Ok(StatusCode::NOT_FOUND);
    };

    if claims.role != "admin" {
        let owned = item.queued_by.as_deref() == Some(claims.sub.as_str());
        if !owned {
            return Ok(StatusCode::FORBIDDEN);
        }
    }

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
