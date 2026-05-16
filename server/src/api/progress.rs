use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::{IntoResponse, Json, Response},
    routing::get,
    Router,
};
use chrono::Utc;
use serde::{Deserialize, Serialize};

use crate::{auth, db::ReadProgress, AppState};
use super::ApiResult;

#[derive(Deserialize)]
pub struct UpdateProgress {
    pub current_page: i64,
    pub completed: bool,
}

#[derive(Debug, Serialize)]
pub struct ContinueItem {
    pub manga_id: String,
    pub manga_title: String,
    pub cover_url: Option<String>,
    pub chapter_id: String,
    pub chapter_number: f64,
    pub chapters_read: i64,
    pub total_chapters: i64,
}

pub fn router() -> Router<AppState> {
    Router::new()
        // /progress/continue must come before /progress/{chapter_id}
        .route("/progress/continue", get(continue_reading))
        .route("/progress/manga/{manga_id}", get(list_manga_progress))
        .route("/progress/{chapter_id}", get(get_progress).put(update_progress))
}

async fn list_manga_progress(
    State(state): State<AppState>,
    axum::Extension(claims): axum::Extension<auth::Claims>,
    Path(manga_id): Path<String>,
) -> ApiResult<Json<Vec<ReadProgress>>> {
    let user_id = &claims.sub;
    let rows = sqlx::query_as!(
        ReadProgress,
        r#"SELECT rp.id as "id!",
                  rp.user_id as "user_id!",
                  rp.chapter_id as "chapter_id!",
                  rp.current_page,
                  (rp.completed != 0) as "completed!: bool",
                  rp.updated_at as "updated_at: _"
           FROM read_progress rp
           JOIN chapters c ON c.id = rp.chapter_id
           WHERE c.manga_id = ? AND rp.user_id = ?"#,
        manga_id,
        user_id
    )
    .fetch_all(&state.db)
    .await?;

    Ok(Json(rows))
}

async fn get_progress(
    State(state): State<AppState>,
    axum::Extension(claims): axum::Extension<auth::Claims>,
    Path(chapter_id): Path<String>,
) -> ApiResult<Response> {
    let user_id = &claims.sub;
    let progress = sqlx::query_as!(
        ReadProgress,
        r#"SELECT
               id as "id!",
               user_id as "user_id!",
               chapter_id as "chapter_id!",
               current_page,
               (completed != 0) as "completed!: bool",
               updated_at as "updated_at: _"
           FROM read_progress WHERE chapter_id = ? AND user_id = ?"#,
        chapter_id,
        user_id
    )
    .fetch_optional(&state.db)
    .await?;

    let Some(progress) = progress else {
        return Ok(StatusCode::NOT_FOUND.into_response());
    };
    Ok(Json(progress).into_response())
}

async fn continue_reading(
    State(state): State<AppState>,
    axum::Extension(claims): axum::Extension<auth::Claims>,
) -> ApiResult<Json<Vec<ContinueItem>>> {
    let user_id = &claims.sub;

    struct Row {
        manga_id: String,
        manga_title: String,
        cover_url: Option<String>,
        chapter_id: Option<String>,
        chapter_number: Option<f64>,
        chapters_read: i64,
        total_chapters: i64,
    }

    let rows = sqlx::query_as!(
        Row,
        r#"SELECT
               m.id          as "manga_id!",
               m.title       as "manga_title!",
               m.cover_url,
               (SELECT c.id FROM chapters c
                LEFT JOIN read_progress rp ON rp.chapter_id = c.id AND rp.user_id = ?
                WHERE c.manga_id = m.id AND c.downloaded = 1
                  AND (rp.completed IS NULL OR rp.completed = 0)
                ORDER BY c.number ASC LIMIT 1)   as chapter_id,
               (SELECT c.number FROM chapters c
                LEFT JOIN read_progress rp ON rp.chapter_id = c.id AND rp.user_id = ?
                WHERE c.manga_id = m.id AND c.downloaded = 1
                  AND (rp.completed IS NULL OR rp.completed = 0)
                ORDER BY c.number ASC LIMIT 1)   as "chapter_number: f64",
               (SELECT COUNT(*) FROM read_progress rp2
                JOIN chapters c2 ON c2.id = rp2.chapter_id
                WHERE c2.manga_id = m.id AND rp2.completed = 1 AND rp2.user_id = ?) as "chapters_read!: i64",
               (SELECT COUNT(*) FROM chapters c3
                WHERE c3.manga_id = m.id)         as "total_chapters!: i64"
           FROM manga m
           WHERE EXISTS (
               SELECT 1 FROM read_progress rp4
               JOIN chapters c4 ON c4.id = rp4.chapter_id
               WHERE c4.manga_id = m.id AND rp4.completed = 1 AND rp4.user_id = ?
           )
           AND EXISTS (
               SELECT 1 FROM chapters c5
               LEFT JOIN read_progress rp5 ON rp5.chapter_id = c5.id AND rp5.user_id = ?
               WHERE c5.manga_id = m.id AND c5.downloaded = 1
                 AND (rp5.completed IS NULL OR rp5.completed = 0)
           )
           ORDER BY m.updated_at DESC
           LIMIT 10"#,
        user_id, user_id, user_id, user_id, user_id
    )
    .fetch_all(&state.db)
    .await?;

    let items = rows
        .into_iter()
        .filter_map(|r| {
            Some(ContinueItem {
                manga_id: r.manga_id,
                manga_title: r.manga_title,
                cover_url: r.cover_url,
                chapter_id: r.chapter_id?,
                chapter_number: r.chapter_number?,
                chapters_read: r.chapters_read,
                total_chapters: r.total_chapters,
            })
        })
        .collect();

    Ok(Json(items))
}

async fn update_progress(
    State(state): State<AppState>,
    axum::Extension(claims): axum::Extension<auth::Claims>,
    Path(chapter_id): Path<String>,
    Json(body): Json<UpdateProgress>,
) -> ApiResult<Json<ReadProgress>> {
    let user_id = &claims.sub;
    let id = uuid::Uuid::new_v4().to_string();
    let now = Utc::now();
    let completed = body.completed as i64;

    sqlx::query!(
        r#"INSERT INTO read_progress (id, user_id, chapter_id, current_page, completed, updated_at)
           VALUES (?, ?, ?, ?, ?, ?)
           ON CONFLICT(user_id, chapter_id) DO UPDATE SET
             current_page = excluded.current_page,
             completed = excluded.completed,
             updated_at = excluded.updated_at"#,
        id, user_id, chapter_id, body.current_page, completed, now
    )
    .execute(&state.db)
    .await?;

    let progress = sqlx::query_as!(
        ReadProgress,
        r#"SELECT
               id as "id!",
               user_id as "user_id!",
               chapter_id as "chapter_id!",
               current_page,
               (completed != 0) as "completed!: bool",
               updated_at as "updated_at: _"
           FROM read_progress WHERE chapter_id = ? AND user_id = ?"#,
        chapter_id,
        user_id
    )
    .fetch_one(&state.db)
    .await?;

    Ok(Json(progress))
}
