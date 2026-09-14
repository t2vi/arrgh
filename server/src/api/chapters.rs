//! `/api/chapters` — port of `Api/Chapters.cs` (ADR 0033, S5 #127). Flipped
//! in `docker/nginx.conf` as a block with `titles`/`progress`/`queue` now
//! that S7 (#129) landed (see `crate::titles`'s module doc).
//!
//! `VerifyDownloadsAsync` (pruning `downloaded=1` rows whose file vanished)
//! is a no-op TODO stub on the .NET side too — nothing to port yet.

use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::routing::get;
use axum::{Json, Router};
use serde::Serialize;

use crate::auth::Claims;
use crate::chapters;
use crate::error::{AppError, AppResult};
use crate::state::AppState;

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/title/{title_id}", get(list))
        .route("/{id}", get(get_chapter))
        .route("/{id}/text", get(get_chapter_text))
        .route("/{id}/download", axum::routing::post(queue_download))
}

#[derive(Serialize)]
struct ChapterDto {
    id: String,
    title_id: String,
    title: Option<String>,
    number: f64,
    volume: Option<f64>,
    local_path: Option<String>,
    page_count: i64,
    downloaded: bool,
    has_sources: bool,
    chapter_format: String,
    created_at: String,
}

impl From<chapters::ChapterRow> for ChapterDto {
    fn from(c: chapters::ChapterRow) -> Self {
        Self {
            id: c.id,
            title_id: c.title_id,
            title: c.title,
            number: c.number,
            volume: c.volume,
            local_path: c.local_path,
            page_count: c.page_count,
            downloaded: c.downloaded,
            has_sources: c.has_sources,
            chapter_format: c.chapter_format,
            created_at: c.created_at.replacen(' ', "T", 1),
        }
    }
}

async fn list(
    claims: Claims,
    State(state): State<AppState>,
    Path(title_id): Path<String>,
) -> AppResult<Json<Vec<ChapterDto>>> {
    let rows = chapters::list_chapters(&state.db, &title_id, claims.allow_explicit).await?;
    Ok(Json(rows.into_iter().map(ChapterDto::from).collect()))
}

async fn get_chapter(
    claims: Claims,
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> AppResult<Json<ChapterDto>> {
    let c = chapters::get_chapter(&state.db, &id, claims.allow_explicit)
        .await?
        .ok_or(AppError::NotFound)?;
    Ok(Json(ChapterDto::from(c)))
}

#[derive(Serialize)]
struct ChapterTextDto {
    content: String,
}

async fn get_chapter_text(
    claims: Claims,
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> AppResult<Json<ChapterTextDto>> {
    let info = chapters::chapter_text_info(&state.db, &id, claims.allow_explicit)
        .await?
        .ok_or(AppError::NotFound)?;

    if info.chapter_format != "text" {
        return Err(AppError::BadRequest("not a text chapter".into()));
    }
    if !info.downloaded {
        return Err(AppError::NotFound);
    }
    let Some(path) = info.local_path else {
        return Err(AppError::NotFound);
    };

    let content = tokio::fs::read_to_string(&path)
        .await
        .map_err(|_| AppError::NotFound)?;
    Ok(Json(ChapterTextDto { content }))
}

async fn queue_download(
    claims: Claims,
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> AppResult<StatusCode> {
    let candidate = chapters::queue_candidate(&state.db, &id, claims.allow_explicit)
        .await?
        .ok_or(AppError::NotFound)?;

    chapters::queue_download(
        &state.db,
        &candidate.id,
        &candidate.manga_title,
        candidate.number,
        &claims.user_id,
    )
    .await?;

    Ok(StatusCode::ACCEPTED)
}
