use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::{IntoResponse, Json, Response},
    routing::{get, post},
    Router,
};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::{auth, AppState};
use super::ApiResult;

fn ser_opt_bool<S>(v: &Option<i64>, s: S) -> Result<S::Ok, S::Error>
where S: serde::Serializer {
    match v {
        None => s.serialize_none(),
        Some(0) => s.serialize_some(&false),
        Some(_) => s.serialize_some(&true),
    }
}

#[derive(Deserialize)]
pub struct ListQuery {
    pub page: Option<u32>,
    pub limit: Option<u32>,
    pub search: Option<String>,
}

/// Title with aggregated chapter/progress stats for the library grid.
#[derive(Debug, Serialize, sqlx::FromRow)]
pub struct TitleListItem {
    pub id: String,
    pub title: String,
    pub description: Option<String>,
    pub cover_url: Option<String>,
    pub status: String,
    pub is_local: bool,
    pub local_path: Option<String>,
    pub author: Option<String>,
    pub year: Option<i64>,
    pub tags: Option<String>,
    pub sync_status: String,
    pub content_type: String,
    pub is_explicit: bool,
    #[serde(serialize_with = "ser_opt_bool")]
    pub auto_download: Option<i64>,
    pub reader_mode: Option<String>,
    pub download_dir: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub total_chapters: i64,
    pub downloaded_chapters: i64,
    pub chapters_read: i64,
}

#[derive(Serialize)]
pub struct PaginatedTitle {
    pub items: Vec<TitleListItem>,
    pub total: i64,
    pub page: u32,
    pub limit: u32,
}

#[derive(Debug, Serialize, sqlx::FromRow)]
pub struct NewReleaseItem {
    pub chapter_id: String,
    pub chapter_number: f64,
    pub chapter_title: Option<String>,
    pub chapter_created_at: DateTime<Utc>,
    pub downloaded: bool,
    pub manga_id: String,
    pub manga_title: String,
    pub cover_url: Option<String>,
}

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/titles", get(list_titles))
        .route("/titles/new-releases", get(new_releases))
        .route("/titles/{id}", get(get_title).delete(remove_title).patch(patch_title))
        .route("/titles/{id}/sync", post(sync_title))
}

async fn list_titles(
    State(state): State<AppState>,
    axum::Extension(claims): axum::Extension<auth::Claims>,
    Query(query): Query<ListQuery>,
) -> ApiResult<Json<PaginatedTitle>> {
    let page = query.page.unwrap_or(1).max(1);
    let limit = query.limit.unwrap_or(20).min(100);
    let offset = ((page - 1) * limit) as i64;
    let limit_i64 = limit as i64;
    let user_id = &claims.sub;
    let allow_explicit: i64 = if claims.allow_explicit { 1 } else { 0 };

    let (items, total) = if let Some(search) = &query.search {
        let pattern = format!("%{}%", search);
        let items = sqlx::query_as!(
            TitleListItem,
            r#"SELECT
                   m.id as "id!",
                   m.title as "title!",
                   m.description,
                   m.cover_url,
                   m.status as "status!",
                   (NOT EXISTS(SELECT 1 FROM title_sources ts WHERE ts.title_id = m.id)) as "is_local!: bool",
                   m.local_path,
                   m.author,
                   m.year,
                   m.tags,
                   m.sync_status as "sync_status!",
                   m.content_type as "content_type!",
                   (m.is_explicit != 0) as "is_explicit!: bool",
                   m.auto_download,
                   (SELECT ums.reader_mode FROM user_title_settings ums
                    WHERE ums.title_id = m.id AND ums.user_id = ?) as reader_mode,
                   m.download_dir,
                   m.created_at as "created_at: _",
                   m.updated_at as "updated_at: _",
                   (SELECT COUNT(*) FROM chapters WHERE title_id = m.id) as "total_chapters!: i64",
                   (SELECT COUNT(*) FROM chapters WHERE title_id = m.id AND downloaded = 1) as "downloaded_chapters!: i64",
                   (SELECT COUNT(*) FROM read_progress rp JOIN chapters c ON c.id = rp.chapter_id
                    WHERE c.title_id = m.id AND rp.completed = 1 AND rp.user_id = ?) as "chapters_read!: i64"
               FROM titles m
               WHERE m.title LIKE ?
                 AND EXISTS (SELECT 1 FROM user_titles ut WHERE ut.title_id = m.id AND ut.user_id = ?)
                 AND (m.is_explicit = 0 OR ? = 1)
               ORDER BY m.title LIMIT ? OFFSET ?"#,
            user_id, user_id, pattern, user_id, allow_explicit, limit_i64, offset
        )
        .fetch_all(&state.db)
        .await?;

        let total: i64 = sqlx::query_scalar!(
            "SELECT COUNT(*) FROM titles m WHERE m.title LIKE ? AND EXISTS (SELECT 1 FROM user_titles ut WHERE ut.title_id = m.id AND ut.user_id = ?) AND (m.is_explicit = 0 OR ? = 1)",
            pattern, user_id, allow_explicit
        )
        .fetch_one(&state.db)
        .await?;

        (items, total)
    } else {
        let items = sqlx::query_as!(
            TitleListItem,
            r#"SELECT
                   m.id as "id!",
                   m.title as "title!",
                   m.description,
                   m.cover_url,
                   m.status as "status!",
                   (NOT EXISTS(SELECT 1 FROM title_sources ts WHERE ts.title_id = m.id)) as "is_local!: bool",
                   m.local_path,
                   m.author,
                   m.year,
                   m.tags,
                   m.sync_status as "sync_status!",
                   m.content_type as "content_type!",
                   (m.is_explicit != 0) as "is_explicit!: bool",
                   m.auto_download,
                   (SELECT ums.reader_mode FROM user_title_settings ums
                    WHERE ums.title_id = m.id AND ums.user_id = ?) as reader_mode,
                   m.download_dir,
                   m.created_at as "created_at: _",
                   m.updated_at as "updated_at: _",
                   (SELECT COUNT(*) FROM chapters WHERE title_id = m.id) as "total_chapters!: i64",
                   (SELECT COUNT(*) FROM chapters WHERE title_id = m.id AND downloaded = 1) as "downloaded_chapters!: i64",
                   (SELECT COUNT(*) FROM read_progress rp JOIN chapters c ON c.id = rp.chapter_id
                    WHERE c.title_id = m.id AND rp.completed = 1 AND rp.user_id = ?) as "chapters_read!: i64"
               FROM titles m
               WHERE EXISTS (SELECT 1 FROM user_titles ut WHERE ut.title_id = m.id AND ut.user_id = ?)
                 AND (m.is_explicit = 0 OR ? = 1)
               ORDER BY m.updated_at DESC LIMIT ? OFFSET ?"#,
            user_id, user_id, user_id, allow_explicit, limit_i64, offset
        )
        .fetch_all(&state.db)
        .await?;

        let total: i64 = sqlx::query_scalar!(
            "SELECT COUNT(*) FROM titles m WHERE EXISTS (SELECT 1 FROM user_titles ut WHERE ut.title_id = m.id AND ut.user_id = ?) AND (m.is_explicit = 0 OR ? = 1)",
            user_id, allow_explicit
        )
        .fetch_one(&state.db)
        .await?;

        (items, total)
    };

    Ok(Json(PaginatedTitle { items, total, page, limit }))
}

async fn get_title(
    State(state): State<AppState>,
    axum::Extension(claims): axum::Extension<auth::Claims>,
    Path(id): Path<String>,
) -> ApiResult<Response> {
    let user_id = &claims.sub;
    let allow_explicit: i64 = if claims.allow_explicit { 1 } else { 0 };

    let title = sqlx::query_as!(
        TitleListItem,
        r#"SELECT
               m.id as "id!",
               m.title as "title!",
               m.description,
               m.cover_url,
               m.status as "status!",
               (NOT EXISTS(SELECT 1 FROM title_sources ts WHERE ts.title_id = m.id)) as "is_local!: bool",
               m.local_path,
               m.author,
               m.year,
               m.tags,
               m.sync_status as "sync_status!",
               m.content_type as "content_type!",
               (m.is_explicit != 0) as "is_explicit!: bool",
               m.auto_download,
               (SELECT ums.reader_mode FROM user_title_settings ums
                WHERE ums.title_id = m.id AND ums.user_id = ?) as reader_mode,
               m.download_dir,
               m.created_at as "created_at: _",
               m.updated_at as "updated_at: _",
               (SELECT COUNT(*) FROM chapters WHERE title_id = m.id) as "total_chapters!: i64",
               (SELECT COUNT(*) FROM chapters WHERE title_id = m.id AND downloaded = 1) as "downloaded_chapters!: i64",
               (SELECT COUNT(*) FROM read_progress rp JOIN chapters c ON c.id = rp.chapter_id
                WHERE c.title_id = m.id AND rp.completed = 1 AND rp.user_id = ?) as "chapters_read!: i64"
           FROM titles m
           WHERE m.id = ?
             AND EXISTS (SELECT 1 FROM user_titles ut WHERE ut.title_id = m.id AND ut.user_id = ?)
             AND (m.is_explicit = 0 OR ? = 1)"#,
        user_id, user_id, id, user_id, allow_explicit
    )
    .fetch_optional(&state.db)
    .await?;

    let Some(title) = title else {
        return Ok(StatusCode::NOT_FOUND.into_response());
    };
    Ok(Json(title).into_response())
}

#[derive(Deserialize)]
struct RemoveQuery {
    delete_files: Option<bool>,
}

async fn remove_title(
    State(state): State<AppState>,
    axum::Extension(claims): axum::Extension<auth::Claims>,
    Path(id): Path<String>,
    Query(q): Query<RemoveQuery>,
) -> ApiResult<StatusCode> {
    let result = sqlx::query!(
        "DELETE FROM user_titles WHERE user_id = ? AND title_id = ?",
        claims.sub, id
    )
    .execute(&state.db)
    .await?;

    if result.rows_affected() == 0 {
        return Ok(StatusCode::NOT_FOUND);
    }

    let remaining: i64 = sqlx::query_scalar!(
        "SELECT COUNT(*) FROM user_titles WHERE title_id = ?",
        id
    )
    .fetch_one(&state.db)
    .await?;

    if remaining == 0 {
        if q.delete_files.unwrap_or(false) && claims.role == "admin" {
            let paths = sqlx::query_scalar!(
                "SELECT local_path FROM chapters WHERE title_id = ? AND local_path IS NOT NULL",
                id
            )
            .fetch_all(&state.db)
            .await
            .unwrap_or_default();

            let cover = sqlx::query_scalar!("SELECT cover_url FROM titles WHERE id = ?", id)
                .fetch_optional(&state.db)
                .await
                .ok()
                .flatten()
                .flatten();

            for path in paths.into_iter().flatten() {
                if !path.starts_with("http") {
                    let _ = tokio::fs::remove_file(&path).await;
                }
            }
            if let Some(cover_path) = cover {
                if !cover_path.starts_with("http") {
                    let _ = tokio::fs::remove_file(&cover_path).await;
                }
            }
        }

        sqlx::query!("DELETE FROM titles WHERE id = ?", id)
            .execute(&state.db)
            .await?;
    }

    Ok(StatusCode::NO_CONTENT)
}

async fn new_releases(
    State(state): State<AppState>,
    axum::Extension(claims): axum::Extension<auth::Claims>,
) -> ApiResult<Json<Vec<NewReleaseItem>>> {
    let user_id = &claims.sub;
    let allow_explicit: i64 = if claims.allow_explicit { 1 } else { 0 };

    let items = sqlx::query_as!(
        NewReleaseItem,
        r#"SELECT
               c.id          as "chapter_id!",
               c.number      as "chapter_number!: f64",
               c.title       as chapter_title,
               c.created_at  as "chapter_created_at!: DateTime<Utc>",
               c.downloaded  as "downloaded!: bool",
               m.id          as "manga_id!",
               m.title       as "manga_title!",
               m.cover_url
           FROM chapters c
           JOIN titles m ON c.title_id = m.id
           WHERE c.is_new = 1
             AND EXISTS (SELECT 1 FROM user_titles ut WHERE ut.title_id = m.id AND ut.user_id = ?)
             AND (m.is_explicit = 0 OR ? = 1)
           ORDER BY c.created_at DESC
           LIMIT 30"#,
        user_id, allow_explicit
    )
    .fetch_all(&state.db)
    .await?;

    Ok(Json(items))
}

#[derive(Deserialize)]
struct PatchTitleBody {
    #[serde(default)]
    auto_download: Option<bool>,
    #[serde(default)]
    reader_mode: Option<Option<String>>,
    #[serde(default)]
    download_dir: Option<Option<String>>,
    #[serde(default)]
    is_explicit: Option<bool>,
    #[serde(default)]
    cover_url: Option<String>,
}

async fn patch_title(
    State(state): State<AppState>,
    axum::Extension(claims): axum::Extension<auth::Claims>,
    Path(id): Path<String>,
    Json(body): Json<PatchTitleBody>,
) -> ApiResult<StatusCode> {
    let owns: i64 = sqlx::query_scalar!(
        "SELECT COUNT(*) FROM user_titles WHERE user_id = ? AND title_id = ?",
        claims.sub, id
    )
    .fetch_one(&state.db)
    .await?;
    if owns == 0 {
        return Ok(StatusCode::NOT_FOUND);
    }

    if let Some(ad) = body.auto_download {
        let val: i64 = if ad { 1 } else { 0 };
        sqlx::query!("UPDATE titles SET auto_download = ? WHERE id = ?", val, id)
            .execute(&state.db)
            .await?;
    }
    if let Some(rm) = body.reader_mode {
        if let Some(ref m) = rm {
            if !matches!(m.as_str(), "paged" | "scroll") {
                return Ok(StatusCode::UNPROCESSABLE_ENTITY);
            }
        }
        let user_id = &claims.sub;
        sqlx::query!(
            r#"INSERT INTO user_title_settings (user_id, title_id, reader_mode)
               VALUES (?, ?, ?)
               ON CONFLICT(user_id, title_id) DO UPDATE SET reader_mode = excluded.reader_mode"#,
            user_id, id, rm
        )
        .execute(&state.db)
        .await?;
    }
    if let Some(dd) = body.download_dir {
        sqlx::query!("UPDATE titles SET download_dir = ? WHERE id = ?", dd, id)
            .execute(&state.db)
            .await?;
    }
    if let Some(explicit) = body.is_explicit {
        if claims.role != "admin" {
            return Ok(StatusCode::FORBIDDEN);
        }
        let val: i64 = if explicit { 1 } else { 0 };
        sqlx::query!("UPDATE titles SET is_explicit = ? WHERE id = ?", val, id)
            .execute(&state.db)
            .await?;
    }
    if let Some(url) = body.cover_url {
        if claims.role != "admin" {
            return Ok(StatusCode::FORBIDDEN);
        }
        if url.is_empty() {
            sqlx::query!("UPDATE titles SET cover_url = NULL WHERE id = ?", id)
                .execute(&state.db)
                .await?;
        } else {
            sqlx::query!("UPDATE titles SET cover_url = ? WHERE id = ?", url, id)
                .execute(&state.db)
                .await?;
        }
    }
    Ok(StatusCode::NO_CONTENT)
}

async fn sync_title(
    State(state): State<AppState>,
    axum::Extension(claims): axum::Extension<auth::Claims>,
    Path(id): Path<String>,
) -> ApiResult<StatusCode> {
    let exists = sqlx::query!(
        r#"SELECT id as "id!" FROM titles m WHERE m.id = ? AND EXISTS (SELECT 1 FROM user_titles ut WHERE ut.title_id = m.id AND ut.user_id = ?)"#,
        id, claims.sub
    )
    .fetch_optional(&state.db)
    .await?;

    if exists.is_none() {
        return Ok(StatusCode::NOT_FOUND);
    }

    let source_links = sqlx::query!(
        r#"SELECT source as "source!", source_id as "source_id!" FROM title_sources WHERE title_id = ?"#,
        id
    )
    .fetch_all(&state.db)
    .await?;

    if source_links.is_empty() {
        return Ok(StatusCode::NOT_FOUND);
    }

    sqlx::query!("UPDATE titles SET sync_status = 'syncing' WHERE id = ?", id)
        .execute(&state.db)
        .await?;

    let db = state.db.clone();
    let state_sources = state.sources.clone();
    tokio::spawn(async move {
        let mut any_ok = false;
        for link in &source_links {
            if let Some(s) = state_sources.read().await.get(&link.source).cloned() {
                match s.sync_chapters(&db, &id, &link.source_id).await {
                    Ok(_) => { any_ok = true; }
                    Err(e) => tracing::error!("sync error for title {} ({}): {}", id, link.source, e),
                }
            }
        }
        let status = if any_ok { "ready" } else { "error" };
        let _ = sqlx::query!("UPDATE titles SET sync_status = ? WHERE id = ?", status, id)
            .execute(&db)
            .await;
    });

    Ok(StatusCode::ACCEPTED)
}
