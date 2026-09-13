//! `/api/progress` — port of `Api/Progress.cs` (ADR 0033, S4 #126). Not yet
//! flipped in `docker/nginx.conf` — see `crate::titles`'s module doc (moves
//! as a contiguous block with `titles`).

use axum::extract::{Path, State};
use axum::routing::get;
use axum::{Json, Router};
use serde::{Deserialize, Serialize};

use crate::auth::Claims;
use crate::error::AppResult;
use crate::progress;
use crate::state::AppState;

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/continue", get(continue_reading))
        .route("/title/{title_id}", get(list_title_progress))
        .route("/{chapter_id}", get(get_progress).put(update_progress))
}

#[derive(Serialize)]
struct ReadProgressDto {
    id: String,
    user_id: String,
    chapter_id: String,
    current_page: i64,
    completed: bool,
    updated_at: String,
}

impl From<progress::ReadProgressRow> for ReadProgressDto {
    fn from(r: progress::ReadProgressRow) -> Self {
        Self {
            id: r.id,
            user_id: r.user_id,
            chapter_id: r.chapter_id,
            current_page: r.current_page,
            completed: r.completed,
            updated_at: r.updated_at.replacen(' ', "T", 1),
        }
    }
}

#[derive(Serialize)]
struct ContinueItemDto {
    title_id: String,
    manga_title: String,
    cover_url: Option<String>,
    chapter_id: Option<String>,
    chapter_number: Option<f64>,
    chapters_read: i64,
    total_chapters: i64,
}

impl From<progress::ContinueItem> for ContinueItemDto {
    fn from(c: progress::ContinueItem) -> Self {
        Self {
            title_id: c.title_id,
            manga_title: c.manga_title,
            cover_url: c.cover_url,
            chapter_id: c.chapter_id,
            chapter_number: c.chapter_number,
            chapters_read: c.chapters_read,
            total_chapters: c.total_chapters,
        }
    }
}

async fn list_title_progress(
    claims: Claims,
    State(state): State<AppState>,
    Path(title_id): Path<String>,
) -> AppResult<Json<Vec<ReadProgressDto>>> {
    let rows = progress::list_title_progress(&state.db, &claims.user_id, &title_id).await?;
    Ok(Json(rows.into_iter().map(ReadProgressDto::from).collect()))
}

async fn get_progress(
    claims: Claims,
    State(state): State<AppState>,
    Path(chapter_id): Path<String>,
) -> AppResult<Json<ReadProgressDto>> {
    let row = progress::get_progress(&state.db, &chapter_id, &claims.user_id)
        .await?
        .ok_or(crate::error::AppError::NotFound)?;
    Ok(Json(ReadProgressDto::from(row)))
}

async fn continue_reading(
    claims: Claims,
    State(state): State<AppState>,
) -> AppResult<Json<Vec<ContinueItemDto>>> {
    let items = progress::continue_reading(&state.db, &claims.user_id).await?;
    Ok(Json(items.into_iter().map(ContinueItemDto::from).collect()))
}

#[derive(Deserialize)]
struct UpdateProgressBody {
    current_page: i64,
    completed: bool,
}

async fn update_progress(
    claims: Claims,
    State(state): State<AppState>,
    Path(chapter_id): Path<String>,
    Json(body): Json<UpdateProgressBody>,
) -> AppResult<Json<ReadProgressDto>> {
    let row = progress::upsert_progress(
        &state.db,
        &claims.user_id,
        &chapter_id,
        body.current_page,
        body.completed,
    )
    .await?;
    Ok(Json(ReadProgressDto::from(row)))
}
