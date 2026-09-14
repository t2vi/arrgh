//! `/api/queue` — port of `Api/Queue.cs` (ADR 0033, S7 #129). Flips
//! nginx together with `titles`/`chapters`/`progress` — see
//! `crate::titles`'s module doc; this phase completes that block.

use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::routing::get;
use axum::{Json, Router};
use serde::Serialize;

use crate::auth::Claims;
use crate::error::{AppError, AppResult};
use crate::queue;
use crate::state::AppState;

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/", get(list))
        .route("/title/{title_id}", get(list_for_title))
        .route("/completed", axum::routing::delete(clear_completed))
        .route("/{id}", axum::routing::delete(remove))
}

#[derive(Serialize)]
struct QueueItemDto {
    id: String,
    chapter_id: String,
    manga_title: String,
    chapter_num: f64,
    status: String,
    error: Option<String>,
    pages_downloaded: i64,
    pages_total: i64,
    created_at: String,
    updated_at: String,
}

impl From<queue::QueueItemRow> for QueueItemDto {
    fn from(q: queue::QueueItemRow) -> Self {
        Self {
            id: q.id,
            chapter_id: q.chapter_id,
            manga_title: q.manga_title,
            chapter_num: q.chapter_num,
            status: q.status,
            error: q.error,
            pages_downloaded: q.pages_downloaded,
            pages_total: q.pages_total,
            created_at: q.created_at.replacen(' ', "T", 1),
            updated_at: q.updated_at.replacen(' ', "T", 1),
        }
    }
}

fn allowed_explicit(claims: &Claims) -> bool {
    queue::is_allowed_explicit(&claims.role, claims.allow_explicit)
}

async fn list(claims: Claims, State(state): State<AppState>) -> AppResult<Json<Vec<QueueItemDto>>> {
    let rows = queue::list(&state.db, allowed_explicit(&claims)).await?;
    Ok(Json(rows.into_iter().map(QueueItemDto::from).collect()))
}

async fn list_for_title(
    claims: Claims,
    State(state): State<AppState>,
    Path(title_id): Path<String>,
) -> AppResult<Json<Vec<QueueItemDto>>> {
    let rows = queue::list_for_title(&state.db, &title_id, allowed_explicit(&claims)).await?;
    Ok(Json(rows.into_iter().map(QueueItemDto::from).collect()))
}

async fn clear_completed(claims: Claims, State(state): State<AppState>) -> AppResult<StatusCode> {
    claims.require_admin()?;
    queue::clear_completed(&state.db).await?;
    Ok(StatusCode::NO_CONTENT)
}

async fn remove(
    claims: Claims,
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> AppResult<StatusCode> {
    let item = queue::get_ownership(&state.db, &id)
        .await?
        .ok_or(AppError::NotFound)?;

    let is_admin = claims.role == "admin";
    if !is_admin && item.queued_by.as_deref() != Some(claims.user_id.as_str()) {
        return Err(AppError::Forbidden);
    }

    queue::remove_or_cancel(&state.db, &id).await?;
    Ok(StatusCode::NO_CONTENT)
}
