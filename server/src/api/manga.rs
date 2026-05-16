use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::{IntoResponse, Json, Response},
    routing::{get, post},
    Router,
};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::AppState;
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

/// Manga with aggregated chapter/progress stats for the library grid.
#[derive(Debug, Serialize, sqlx::FromRow)]
pub struct MangaListItem {
    pub id: String,
    pub title: String,
    pub description: Option<String>,
    pub cover_url: Option<String>,
    pub status: String,
    pub source: String,
    pub source_id: Option<String>,
    pub local_path: Option<String>,
    pub author: Option<String>,
    pub year: Option<i64>,
    pub tags: Option<String>,
    pub sync_status: String,
    pub content_type: String,
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
pub struct PaginatedManga {
    pub items: Vec<MangaListItem>,
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
        .route("/manga", get(list_manga))
        .route("/manga/new-releases", get(new_releases))
        .route("/manga/{id}", get(get_manga).delete(remove_manga).patch(patch_manga))
        .route("/manga/{id}/sync", post(sync_manga))
}

async fn list_manga(
    State(state): State<AppState>,
    Query(query): Query<ListQuery>,
) -> ApiResult<Json<PaginatedManga>> {
    let page = query.page.unwrap_or(1).max(1);
    let limit = query.limit.unwrap_or(20).min(100);
    let offset = ((page - 1) * limit) as i64;
    let limit_i64 = limit as i64;

    let (items, total) = if let Some(search) = &query.search {
        let pattern = format!("%{}%", search);
        let items = sqlx::query_as!(
            MangaListItem,
            r#"SELECT
                   m.id as "id!",
                   m.title as "title!",
                   m.description,
                   m.cover_url,
                   m.status as "status!",
                   m.source as "source!",
                   m.source_id,
                   m.local_path,
                   m.author,
                   m.year,
                   m.tags,
                   m.sync_status as "sync_status!",
                   m.content_type as "content_type!",
                   m.auto_download,
                   m.reader_mode,
                   m.download_dir,
                   m.created_at as "created_at: _",
                   m.updated_at as "updated_at: _",
                   (SELECT COUNT(*) FROM chapters WHERE manga_id = m.id) as "total_chapters!: i64",
                   (SELECT COUNT(*) FROM chapters WHERE manga_id = m.id AND downloaded = 1) as "downloaded_chapters!: i64",
                   (SELECT COUNT(*) FROM read_progress rp JOIN chapters c ON c.id = rp.chapter_id
                    WHERE c.manga_id = m.id AND rp.completed = 1) as "chapters_read!: i64"
               FROM manga m WHERE m.title LIKE ? ORDER BY m.title LIMIT ? OFFSET ?"#,
            pattern, limit_i64, offset
        )
        .fetch_all(&state.db)
        .await?;

        let total: i64 = sqlx::query_scalar!(
            "SELECT COUNT(*) FROM manga WHERE title LIKE ?",
            pattern
        )
        .fetch_one(&state.db)
        .await?;

        (items, total)
    } else {
        let items = sqlx::query_as!(
            MangaListItem,
            r#"SELECT
                   m.id as "id!",
                   m.title as "title!",
                   m.description,
                   m.cover_url,
                   m.status as "status!",
                   m.source as "source!",
                   m.source_id,
                   m.local_path,
                   m.author,
                   m.year,
                   m.tags,
                   m.sync_status as "sync_status!",
                   m.content_type as "content_type!",
                   m.auto_download,
                   m.reader_mode,
                   m.download_dir,
                   m.created_at as "created_at: _",
                   m.updated_at as "updated_at: _",
                   (SELECT COUNT(*) FROM chapters WHERE manga_id = m.id) as "total_chapters!: i64",
                   (SELECT COUNT(*) FROM chapters WHERE manga_id = m.id AND downloaded = 1) as "downloaded_chapters!: i64",
                   (SELECT COUNT(*) FROM read_progress rp JOIN chapters c ON c.id = rp.chapter_id
                    WHERE c.manga_id = m.id AND rp.completed = 1) as "chapters_read!: i64"
               FROM manga m ORDER BY m.updated_at DESC LIMIT ? OFFSET ?"#,
            limit_i64, offset
        )
        .fetch_all(&state.db)
        .await?;

        let total: i64 = sqlx::query_scalar!("SELECT COUNT(*) FROM manga")
            .fetch_one(&state.db)
            .await?;

        (items, total)
    };

    Ok(Json(PaginatedManga { items, total, page, limit }))
}

async fn get_manga(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> ApiResult<Response> {
    let manga = sqlx::query_as!(
        MangaListItem,
        r#"SELECT
               m.id as "id!",
               m.title as "title!",
               m.description,
               m.cover_url,
               m.status as "status!",
               m.source as "source!",
               m.source_id,
               m.local_path,
               m.author,
               m.year,
               m.tags,
               m.sync_status as "sync_status!",
               m.content_type as "content_type!",
               m.auto_download,
               m.reader_mode,
               m.download_dir,
               m.created_at as "created_at: _",
               m.updated_at as "updated_at: _",
               (SELECT COUNT(*) FROM chapters WHERE manga_id = m.id) as "total_chapters!: i64",
               (SELECT COUNT(*) FROM chapters WHERE manga_id = m.id AND downloaded = 1) as "downloaded_chapters!: i64",
               (SELECT COUNT(*) FROM read_progress rp JOIN chapters c ON c.id = rp.chapter_id
                WHERE c.manga_id = m.id AND rp.completed = 1) as "chapters_read!: i64"
           FROM manga m WHERE m.id = ?"#,
        id
    )
    .fetch_optional(&state.db)
    .await?;

    let Some(manga) = manga else {
        return Ok(StatusCode::NOT_FOUND.into_response());
    };
    Ok(Json(manga).into_response())
}

#[derive(Deserialize)]
struct RemoveQuery {
    delete_files: Option<bool>,
}

async fn remove_manga(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Query(q): Query<RemoveQuery>,
) -> ApiResult<StatusCode> {
    if q.delete_files.unwrap_or(false) {
        // Collect local files before deleting DB rows
        let paths = sqlx::query_scalar!(
            "SELECT local_path FROM chapters WHERE manga_id = ? AND local_path IS NOT NULL",
            id
        )
        .fetch_all(&state.db)
        .await
        .unwrap_or_default();

        let cover = sqlx::query_scalar!("SELECT cover_url FROM manga WHERE id = ?", id)
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

    let result = sqlx::query!("DELETE FROM manga WHERE id = ?", id)
        .execute(&state.db)
        .await?;

    if result.rows_affected() > 0 {
        Ok(StatusCode::NO_CONTENT)
    } else {
        Ok(StatusCode::NOT_FOUND)
    }
}

async fn new_releases(
    State(state): State<AppState>,
) -> ApiResult<Json<Vec<NewReleaseItem>>> {
    // Chapters inserted more than 1 hour after their manga was added = not from initial sync
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
           JOIN manga m ON c.manga_id = m.id
           WHERE c.is_new = 1
           ORDER BY c.created_at DESC
           LIMIT 30"#
    )
    .fetch_all(&state.db)
    .await?;

    Ok(Json(items))
}

#[derive(Deserialize)]
struct PatchMangaBody {
    #[serde(default)]
    auto_download: Option<bool>,
    #[serde(default)]
    reader_mode: Option<Option<String>>,
    #[serde(default)]
    download_dir: Option<Option<String>>,
}

async fn patch_manga(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Json(body): Json<PatchMangaBody>,
) -> ApiResult<StatusCode> {
    if let Some(ad) = body.auto_download {
        let val: i64 = if ad { 1 } else { 0 };
        sqlx::query!("UPDATE manga SET auto_download = ? WHERE id = ?", val, id)
            .execute(&state.db)
            .await?;
    }
    if let Some(rm) = body.reader_mode {
        if let Some(ref m) = rm {
            if !matches!(m.as_str(), "paged" | "scroll") {
                return Ok(StatusCode::UNPROCESSABLE_ENTITY);
            }
        }
        sqlx::query!("UPDATE manga SET reader_mode = ? WHERE id = ?", rm, id)
            .execute(&state.db)
            .await?;
    }
    if let Some(dd) = body.download_dir {
        sqlx::query!("UPDATE manga SET download_dir = ? WHERE id = ?", dd, id)
            .execute(&state.db)
            .await?;
    }
    Ok(StatusCode::NO_CONTENT)
}

async fn sync_manga(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> ApiResult<StatusCode> {
    let manga = sqlx::query!(
        r#"SELECT source as "source!", source_id FROM manga WHERE id = ?"#,
        id
    )
    .fetch_optional(&state.db)
    .await?;

    let Some(manga) = manga else {
        return Ok(StatusCode::NOT_FOUND);
    };
    let Some(source_id) = manga.source_id else {
        return Ok(StatusCode::NOT_FOUND);
    };

    sqlx::query!("UPDATE manga SET sync_status = 'syncing' WHERE id = ?", id)
        .execute(&state.db)
        .await?;

    let db = state.db.clone();
    let source = manga.source;
    let src = state.sources.get(&source).cloned();
    tokio::spawn(async move {
        let result = match src {
            Some(s) => s.sync_chapters(&db, &id, &source_id).await,
            None => Err(anyhow::anyhow!("unknown source: {}", source)),
        };
        if let Err(e) = &result {
            tracing::error!("sync error: {}", e);
        }
        let status = if result.is_ok() { "ready" } else { "error" };
        let _ = sqlx::query!("UPDATE manga SET sync_status = ? WHERE id = ?", status, id)
            .execute(&db)
            .await;
    });

    Ok(StatusCode::ACCEPTED)
}
