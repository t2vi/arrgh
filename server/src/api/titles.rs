use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::{IntoResponse, Json, Response},
    routing::{get, post},
    Router,
};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::{auth, mangaupdates::MangaUpdatesClient, AppState};
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
    pub has_sync_warnings: bool,
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
        .route("/titles/{id}/sync-log", get(get_sync_log))
        .route("/titles/{id}/refresh-metadata", post(refresh_metadata))
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
                    WHERE c.title_id = m.id AND rp.completed = 1 AND rp.user_id = ?) as "chapters_read!: i64",
                   EXISTS(SELECT 1 FROM sync_warnings sw WHERE sw.title_id = m.id) as "has_sync_warnings!: bool"
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
                    WHERE c.title_id = m.id AND rp.completed = 1 AND rp.user_id = ?) as "chapters_read!: i64",
                   EXISTS(SELECT 1 FROM sync_warnings sw WHERE sw.title_id = m.id) as "has_sync_warnings!: bool"
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
                WHERE c.title_id = m.id AND rp.completed = 1 AND rp.user_id = ?) as "chapters_read!: i64",
               EXISTS(SELECT 1 FROM sync_warnings sw WHERE sw.title_id = m.id) as "has_sync_warnings!: bool"
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
    #[serde(default)]
    content_type: Option<String>,
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
    if let Some(new_ct) = body.content_type {
        if claims.role != "admin" {
            return Ok(StatusCode::FORBIDDEN);
        }
        if !matches!(new_ct.as_str(), "manga" | "manhwa" | "manhua" | "novel") {
            return Ok(StatusCode::UNPROCESSABLE_ENTITY);
        }
        let old_ct = sqlx::query_scalar!(
            r#"SELECT content_type as "content_type!" FROM titles WHERE id = ?"#, id
        )
        .fetch_optional(&state.db)
        .await?;

        let Some(old_ct) = old_ct else {
            return Ok(StatusCode::NOT_FOUND);
        };

        if old_ct != new_ct {
            sqlx::query!("UPDATE titles SET content_type = ? WHERE id = ?", new_ct, id)
                .execute(&state.db)
                .await?;

            // Clear sources + chapters incompatible with new content_type, then re-match
            let db2 = state.db.clone();
            let sources2 = state.sources.clone();
            let http2 = state.http.clone();
            let id2 = id.clone();
            let ct2 = new_ct.clone();
            tokio::spawn(async move {
                use crate::api::discover::{normalize_title, title_matches};
                use std::sync::Arc;

                // Find incompatible title_sources: sources whose content_types don't include new_ct
                let source_links = sqlx::query!(
                    r#"SELECT source as "source!", source_id as "source_id!" FROM title_sources WHERE title_id = ?"#,
                    id2
                )
                .fetch_all(&db2).await.unwrap_or_default();

                // Compatible = supports new content_type AND same explicit tier
                // We don't know has_hentai_tag yet, so we just filter by content_type for the
                // incompatibility check — the re-match below will apply the full filter.
                let compatible_sources: Vec<String> = {
                    let reg = sources2.read().await;
                    reg.iter()
                        .filter(|(_, src)| src.content_types().iter().any(|ct| ct == &ct2))
                        .map(|(id, _)| id.clone())
                        .collect()
                };

                for link in &source_links {
                    if !compatible_sources.contains(&link.source) {
                        // Remove incompatible source link
                        let _ = sqlx::query!(
                            "DELETE FROM title_sources WHERE title_id = ? AND source = ?",
                            id2, link.source
                        ).execute(&db2).await;

                        // Remove chapter_sources for this source
                        let _ = sqlx::query!(
                            "DELETE FROM chapter_sources WHERE source = ? AND chapter_id IN (SELECT id FROM chapters WHERE title_id = ?)",
                            link.source, id2
                        ).execute(&db2).await;
                    }
                }

                // Remove chapters with no remaining chapter_sources
                let _ = sqlx::query!(
                    "DELETE FROM chapters WHERE title_id = ? AND NOT EXISTS (SELECT 1 FROM chapter_sources cs WHERE cs.chapter_id = chapters.id)",
                    id2
                ).execute(&db2).await;

                // Re-run source matching with new content_type
                let title_row = sqlx::query!(
                    r#"SELECT title as "title!", mangaupdates_id, tags FROM titles WHERE id = ?"#, id2
                )
                .fetch_optional(&db2).await.ok().flatten();

                let Some(row) = title_row else { return };

                let has_hentai_tag = row.tags.as_deref()
                    .map(|t| t.split(',').any(|tag| tag.trim().eq_ignore_ascii_case("hentai")))
                    .unwrap_or(false);

                let aliases: Vec<String> = sqlx::query_scalar!(
                    "SELECT alias FROM title_aliases WHERE title_id = ?", id2
                )
                .fetch_all(&db2).await.unwrap_or_default();

                let known_norms: Vec<String> = std::iter::once(normalize_title(&row.title))
                    .chain(aliases.iter().map(|a| normalize_title(a)))
                    .collect();

                let candidates: Vec<String> = std::iter::once(row.title.clone())
                    .chain(aliases.iter().cloned())
                    .collect();

                let source_snapshot: Vec<(String, Arc<dyn crate::indexer::Source>)> = {
                    let reg = sources2.read().await;
                    reg.iter()
                        .filter(|(_, src)| {
                            src.content_types().iter().any(|ct| ct == &ct2)
                                && src.default_explicit() == has_hentai_tag
                        })
                        .map(|(id, src)| (id.clone(), src.clone()))
                        .collect()
                };

                let _ = sqlx::query!("UPDATE titles SET sync_status = 'syncing' WHERE id = ?", id2)
                    .execute(&db2).await;
                let _ = sqlx::query!("DELETE FROM sync_log WHERE title_id = ?", id2)
                    .execute(&db2).await;
                super::append_sync_log(&db2, &id2, "Re-matching sources for new content type…").await;

                let mut any_ok = false;
                for (src_key, src) in &source_snapshot {
                    super::append_sync_log(&db2, &id2, &format!("Searching {}…", src_key)).await;
                    let mut matched_hit: Option<crate::indexer::source::MangaResult> = None;

                    'cands: for candidate in &candidates {
                        let results = match src.search(candidate).await {
                            Ok(r) => r,
                            Err(_) => continue,
                        };
                        for r in &results {
                            if known_norms.iter().any(|kn| title_matches(kn, &normalize_title(&r.title))) {
                                matched_hit = Some(r.clone());
                                break 'cands;
                            }
                        }
                    }

                    let Some(hit) = matched_hit else {
                        super::append_sync_log(&db2, &id2, &format!("No match in {}", src_key)).await;
                        let warn_id = uuid::Uuid::new_v4().to_string();
                        let now_w = chrono::Utc::now().to_rfc3339();
                        let msg = format!("no matching source found for '{}' or any alias", row.title);
                        let _ = sqlx::query!(
                            "INSERT INTO sync_warnings (id, title_id, plugin_id, message, created_at)
                             VALUES (?, ?, ?, ?, ?)
                             ON CONFLICT(title_id, plugin_id) DO UPDATE SET message = excluded.message, created_at = excluded.created_at",
                            warn_id, id2, src_key, msg, now_w
                        ).execute(&db2).await;
                        continue;
                    };

                    super::append_sync_log(&db2, &id2, &format!("Matched in {}: {}", src_key, hit.title)).await;
                    let _ = sqlx::query!(
                        "DELETE FROM sync_warnings WHERE title_id = ? AND plugin_id = ?",
                        id2, src_key
                    ).execute(&db2).await;

                    let ms_id = uuid::Uuid::new_v4().to_string();
                    let disc = chrono::Utc::now().to_rfc3339();
                    let _ = sqlx::query!(
                        "INSERT OR IGNORE INTO title_sources (id, title_id, source, source_id, discovered_at) VALUES (?, ?, ?, ?, ?)",
                        ms_id, id2, src_key, hit.id, disc
                    ).execute(&db2).await;

                    any_ok = true;

                    super::append_sync_log(&db2, &id2, &format!("Syncing chapters from {}…", src_key)).await;
                    match src.sync_chapters(&db2, &id2, &hit.id).await {
                        Ok(n) => {
                            super::append_sync_log(&db2, &id2, &format!("Synced {} chapters from {}", n, src_key)).await;
                        }
                        Err(e) => {
                            tracing::warn!("chapter sync failed for {} ({}): {}", id2, src_key, e);
                            super::append_sync_log(&db2, &id2, &format!("Chapter sync failed for {}: {}", src_key, e)).await;
                            let warn_id = uuid::Uuid::new_v4().to_string();
                            let now_w = chrono::Utc::now().to_rfc3339();
                            let msg = format!("chapter sync failed for '{}': {}", src_key, e);
                            let _ = sqlx::query!(
                                "INSERT INTO sync_warnings (id, title_id, plugin_id, message, created_at)
                                 VALUES (?, ?, ?, ?, ?)
                                 ON CONFLICT(title_id, plugin_id) DO UPDATE SET message = excluded.message, created_at = excluded.created_at",
                                warn_id, id2, src_key, msg, now_w
                            ).execute(&db2).await;
                        }
                    }
                }

                let status = if source_snapshot.is_empty() || any_ok { "ready" } else { "error" };
                let _ = sqlx::query!("UPDATE titles SET sync_status = ? WHERE id = ?", status, id2)
                    .execute(&db2).await;

                let _ = http2; // keep http in scope
            });
        }
    }
    Ok(StatusCode::NO_CONTENT)
}

#[derive(Debug, Serialize, sqlx::FromRow)]
struct SyncLogEntry {
    id: String,
    message: String,
    created_at: DateTime<Utc>,
}

async fn get_sync_log(
    State(state): State<AppState>,
    axum::Extension(claims): axum::Extension<auth::Claims>,
    Path(id): Path<String>,
) -> ApiResult<Response> {
    let owns: i64 = sqlx::query_scalar!(
        "SELECT COUNT(*) FROM user_titles WHERE user_id = ? AND title_id = ?",
        claims.sub, id
    )
    .fetch_one(&state.db)
    .await?;

    if owns == 0 {
        return Ok(StatusCode::NOT_FOUND.into_response());
    }

    let entries = sqlx::query_as!(
        SyncLogEntry,
        r#"SELECT id as "id!", message as "message!", created_at as "created_at: _"
           FROM sync_log WHERE title_id = ? ORDER BY created_at ASC"#,
        id
    )
    .fetch_all(&state.db)
    .await?;

    Ok(Json(entries).into_response())
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

    sqlx::query!("DELETE FROM sync_log WHERE title_id = ?", id)
        .execute(&state.db)
        .await?;

    let db = state.db.clone();
    let state_sources = state.sources.clone();
    tokio::spawn(async move {
        // Title may have been deleted while we were waiting to run
        let still_exists: bool = sqlx::query_scalar!(
            r#"SELECT COUNT(*) as "c!: i64" FROM titles WHERE id = ?"#, id
        )
        .fetch_one(&db)
        .await
        .map(|c| c > 0)
        .unwrap_or(false);
        if !still_exists { return; }

        let mut any_ok = false;
        for link in &source_links {
            if let Some(s) = state_sources.read().await.get(&link.source).cloned() {
                any_ok = true;

                super::append_sync_log(&db, &id, &format!("Syncing chapters from {}…", link.source)).await;
                match s.sync_chapters(&db, &id, &link.source_id).await {
                    Ok(n) => {
                        super::append_sync_log(&db, &id, &format!("Synced {} chapters from {}", n, link.source)).await;
                    }
                    Err(e) => {
                        tracing::warn!("sync error for title {} ({}): {}", id, link.source, e);
                        super::append_sync_log(&db, &id, &format!("Chapter sync failed for {}: {}", link.source, e)).await;
                        let warn_id = uuid::Uuid::new_v4().to_string();
                        let now_w = chrono::Utc::now().to_rfc3339();
                        let msg = format!("chapter sync failed for '{}': {}", link.source, e);
                        let _ = sqlx::query!(
                            "INSERT INTO sync_warnings (id, title_id, plugin_id, message, created_at)
                             VALUES (?, ?, ?, ?, ?)
                             ON CONFLICT(title_id, plugin_id) DO UPDATE SET message = excluded.message, created_at = excluded.created_at",
                            warn_id, id, link.source, msg, now_w
                        ).execute(&db).await;
                    }
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

async fn refresh_metadata(
    State(state): State<AppState>,
    axum::Extension(claims): axum::Extension<auth::Claims>,
    Path(id): Path<String>,
) -> ApiResult<StatusCode> {
    let row = sqlx::query!(
        r#"SELECT mangaupdates_id FROM titles m
           WHERE m.id = ?
             AND EXISTS (SELECT 1 FROM user_titles ut WHERE ut.title_id = m.id AND ut.user_id = ?)"#,
        id, claims.sub
    )
    .fetch_optional(&state.db)
    .await?;

    let Some(row) = row else {
        return Ok(StatusCode::NOT_FOUND);
    };

    let Some(mu_id_str) = row.mangaupdates_id else {
        return Ok(StatusCode::UNPROCESSABLE_ENTITY);
    };

    let Ok(mu_id) = mu_id_str.parse::<u64>() else {
        return Ok(StatusCode::UNPROCESSABLE_ENTITY);
    };

    let mu = MangaUpdatesClient::new(&state.http);
    let series = match mu.series_detail(mu_id).await? {
        Some(s) => s,
        None => return Ok(StatusCode::NOT_FOUND),
    };

    // Update cover_url from MU if currently missing
    if let Some(ref cover) = series.cover_url {
        sqlx::query!(
            "UPDATE titles SET cover_url = ? WHERE id = ? AND cover_url IS NULL",
            cover, id
        )
        .execute(&state.db)
        .await?;
    }

    // Replace aliases with fresh set
    sqlx::query!("DELETE FROM title_aliases WHERE title_id = ?", id)
        .execute(&state.db)
        .await?;
    for alias in &series.associated_names {
        let alias_id = uuid::Uuid::new_v4().to_string();
        sqlx::query!(
            "INSERT INTO title_aliases (id, title_id, alias) VALUES (?, ?, ?)",
            alias_id, id, alias
        )
        .execute(&state.db)
        .await?;
    }

    // Re-run source matching async for sources with existing warnings
    sqlx::query!("DELETE FROM sync_log WHERE title_id = ?", id)
        .execute(&state.db)
        .await?;

    let db = state.db.clone();
    let sources = state.sources.clone();
    let http = state.http.clone();
    let title_str = series.title.clone();
    let mid = id.clone();
    tokio::spawn(async move {
        use crate::api::discover::normalize_title;
        let aliases: Vec<String> = sqlx::query_scalar!(
            "SELECT alias FROM title_aliases WHERE title_id = ?", mid
        )
        .fetch_all(&db).await.unwrap_or_default();

        // Also include the title stored in the DB — MU's primary title may be in a different
        // language (e.g. Japanese) while the user added the title using its English name.
        let db_title: Option<String> = sqlx::query_scalar!(
            "SELECT title FROM titles WHERE id = ?", mid
        )
        .fetch_optional(&db).await.ok().flatten();

        let all_names: Vec<String> = std::iter::once(title_str.clone())
            .chain(db_title.into_iter().filter(|t| t != &title_str))
            .chain(aliases.iter().cloned())
            .collect();

        let known_norms: Vec<String> = all_names.iter().map(|n| normalize_title(n)).collect();
        let candidates: Vec<String> = all_names;

        // Only retry sources that have warnings (no point re-running sources that already matched)
        let warned_plugins: Vec<String> = sqlx::query_scalar!(
            "SELECT plugin_id FROM sync_warnings WHERE title_id = ?", mid
        )
        .fetch_all(&db).await.unwrap_or_default();

        // _no_source sentinel means no source pool existed at add time — re-run full routing match
        let has_no_source_warning = warned_plugins.iter().any(|p| p == "_no_source");

        let (content_type, has_hentai_tag) = if has_no_source_warning {
            let row = sqlx::query!(
                r#"SELECT content_type as "content_type!", tags FROM titles WHERE id = ?"#, mid
            )
            .fetch_optional(&db).await.ok().flatten();
            let ct = row.as_ref().map(|r| r.content_type.clone()).unwrap_or_else(|| "manga".to_string());
            let hentai = row.and_then(|r| r.tags).map(|t| {
                t.split(',').any(|tag| tag.trim().eq_ignore_ascii_case("hentai"))
            }).unwrap_or(false);
            (ct, hentai)
        } else {
            (String::new(), false)
        };

        let source_snapshot: Vec<(String, std::sync::Arc<dyn crate::indexer::Source>)> = {
            let reg = sources.read().await;
            if has_no_source_warning {
                // Re-run against all routing-eligible sources
                reg.iter()
                    .filter(|(_, src)| {
                        src.content_types().iter().any(|ct| ct == &content_type)
                            && src.default_explicit() == has_hentai_tag
                    })
                    .map(|(id, src)| (id.clone(), src.clone()))
                    .collect()
            } else {
                reg.iter()
                    .filter(|(id, _)| warned_plugins.contains(id))
                    .map(|(id, src)| (id.clone(), src.clone()))
                    .collect()
            }
        };

        super::append_sync_log(&db, &mid, "Searching for sources…").await;
        for (src_key, src) in &source_snapshot {
            super::append_sync_log(&db, &mid, &format!("Searching {}…", src_key)).await;
            let mut matched = None;
            'cands: for candidate in &candidates {
                let results = match src.search(candidate).await {
                    Ok(r) => r,
                    Err(_) => continue,
                };
                for r in &results {
                    if known_norms.iter().any(|kn| {
                        crate::api::discover::title_matches(kn, &normalize_title(&r.title))
                    }) {
                        matched = Some(r.clone());
                        break 'cands;
                    }
                }
            }

            if let Some(hit) = matched {
                super::append_sync_log(&db, &mid, &format!("Matched in {}: {}", src_key, hit.title)).await;

                // Clear per-source warning and sentinel
                let _ = sqlx::query!(
                    "DELETE FROM sync_warnings WHERE title_id = ? AND plugin_id = ?",
                    mid, src_key
                ).execute(&db).await;
                let _ = sqlx::query!(
                    "DELETE FROM sync_warnings WHERE title_id = ? AND plugin_id = '_no_source'",
                    mid
                ).execute(&db).await;

                let ms_id = uuid::Uuid::new_v4().to_string();
                let disc = chrono::Utc::now().to_rfc3339();
                let _ = sqlx::query!(
                    "INSERT OR IGNORE INTO title_sources (id, title_id, source, source_id, discovered_at) VALUES (?, ?, ?, ?, ?)",
                    ms_id, mid, src_key, hit.id, disc
                ).execute(&db).await;

                super::append_sync_log(&db, &mid, &format!("Syncing chapters from {}…", src_key)).await;
                match src.sync_chapters(&db, &mid, &hit.id).await {
                    Ok(n) => super::append_sync_log(&db, &mid, &format!("Synced {} chapters from {}", n, src_key)).await,
                    Err(e) => super::append_sync_log(&db, &mid, &format!("Chapter sync failed for {}: {}", src_key, e)).await,
                }
            } else {
                super::append_sync_log(&db, &mid, &format!("No match in {}", src_key)).await;
            }
        }
        if source_snapshot.is_empty() {
            super::append_sync_log(&db, &mid, "No sources to retry").await;
        }
        let _ = http; // keep http in scope for future use
    });

    Ok(StatusCode::ACCEPTED)
}
