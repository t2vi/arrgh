//! `/api/titles` — port of `Api/Titles.cs` (ADR 0033, S4 #126). Not yet
//! flipped in `docker/nginx.conf` — see `crate::titles`'s module doc.
//!
//! `sync` and `refresh_metadata` both spawn background tasks that mirror the
//! .NET orchestration (status transitions, sync log entries) exactly, and
//! the network legs are real too: chapter-sync since S5 #127
//! (`crate::chapters::sync_from_source`), and Discover re-match + MU alias
//! refresh since S6 #128 (`crate::discover::match_sources`,
//! `crate::metadata::mangaupdates::series_detail`). `patch_title`'s
//! content_type-change branch still just resets `sync_status` to `"ready"`
//! without re-matching sources — matches .NET's own already-stubbed
//! `ReMatchSourcesAsync`, which has the identical TODO.

use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use axum::routing::get;
use axum::{Json, Router};
use serde::{Deserialize, Serialize};
use serde_json::json;

use crate::auth::Claims;
use crate::chapters;
use crate::error::{AppError, AppResult};
use crate::state::AppState;
use crate::titles;

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/", get(list))
        .route("/new-releases", get(new_releases))
        .route(
            "/{id}",
            get(get_title).patch(patch_title).delete(remove_title),
        )
        .route("/{id}/sync", axum::routing::post(sync_title))
        .route("/{id}/sync-log", get(get_sync_log))
        .route(
            "/{id}/refresh-metadata",
            axum::routing::post(refresh_metadata),
        )
}

#[derive(Serialize)]
pub(crate) struct TitleDto {
    id: String,
    title: String,
    description: Option<String>,
    cover_url: Option<String>,
    status: String,
    is_local: bool,
    local_path: Option<String>,
    author: Option<String>,
    year: Option<i64>,
    tags: Option<String>,
    sync_status: String,
    content_type: String,
    is_explicit: bool,
    auto_download: Option<bool>,
    reader_mode: Option<String>,
    download_dir: Option<String>,
    created_at: String,
    updated_at: String,
    total_chapters: i64,
    downloaded_chapters: i64,
    chapters_read: i64,
    has_sync_warnings: bool,
}

impl From<titles::TitleListItem> for TitleDto {
    fn from(t: titles::TitleListItem) -> Self {
        Self {
            id: t.id,
            title: t.title,
            description: t.description,
            cover_url: t.cover_url,
            status: t.status,
            is_local: t.is_local,
            local_path: t.local_path,
            author: t.author,
            year: t.year,
            tags: t.tags,
            sync_status: t.sync_status,
            content_type: t.content_type,
            is_explicit: t.is_explicit,
            auto_download: t.auto_download,
            reader_mode: t.reader_mode,
            download_dir: t.download_dir,
            created_at: t.created_at.replacen(' ', "T", 1),
            updated_at: t.updated_at.replacen(' ', "T", 1),
            total_chapters: t.total_chapters,
            downloaded_chapters: t.downloaded_chapters,
            chapters_read: t.chapters_read,
            has_sync_warnings: t.has_sync_warnings,
        }
    }
}

#[derive(Serialize)]
struct NewReleaseDto {
    chapter_id: String,
    chapter_number: f64,
    chapter_title: Option<String>,
    chapter_created_at: String,
    downloaded: bool,
    manga_id: String,
    manga_title: String,
    cover_url: Option<String>,
}

impl From<titles::NewReleaseItem> for NewReleaseDto {
    fn from(c: titles::NewReleaseItem) -> Self {
        Self {
            chapter_id: c.chapter_id,
            chapter_number: c.chapter_number,
            chapter_title: c.chapter_title,
            chapter_created_at: c.chapter_created_at.replacen(' ', "T", 1),
            downloaded: c.downloaded,
            manga_id: c.manga_id,
            manga_title: c.manga_title,
            cover_url: c.cover_url,
        }
    }
}

// ── GET / ────────────────────────────────────────────────────────────────

#[derive(Deserialize)]
struct ListQuery {
    page: Option<i64>,
    limit: Option<i64>,
    search: Option<String>,
    sort: Option<String>,
    content_type: Option<String>,
    status: Option<String>,
}

async fn list(
    claims: Claims,
    State(state): State<AppState>,
    Query(q): Query<ListQuery>,
) -> AppResult<Json<serde_json::Value>> {
    let page = q.page.unwrap_or(1).max(1);
    let limit = q.limit.unwrap_or(20).min(100);

    let content_types = q
        .content_type
        .as_deref()
        .map(|s| s.split(',').filter(|s| !s.is_empty()).collect::<Vec<_>>())
        .filter(|v| !v.is_empty());
    let statuses = q
        .status
        .as_deref()
        .map(|s| s.split(',').filter(|s| !s.is_empty()).collect::<Vec<_>>())
        .filter(|v| !v.is_empty());
    let params = titles::ListParams {
        search: q.search.as_deref().filter(|s| !s.is_empty()),
        content_types,
        statuses,
    };

    let order_by = match q.sort.as_deref() {
        Some("title_asc") => "t.title ASC",
        Some("title_desc") => "t.title DESC",
        Some("year") => "t.year DESC, t.title ASC",
        Some(_) => "t.created_at DESC",
        None if params.search.is_some() => "t.title ASC",
        None => "t.created_at DESC",
    };

    let total =
        titles::count_titles(&state.db, &claims.user_id, claims.allow_explicit, &params).await?;
    let items = titles::list_titles(
        &state.db,
        &claims.user_id,
        claims.allow_explicit,
        &params,
        order_by,
        limit,
        (page - 1) * limit,
    )
    .await?;
    let items: Vec<TitleDto> = items.into_iter().map(TitleDto::from).collect();

    Ok(Json(
        json!({ "items": items, "total": total, "page": page, "limit": limit }),
    ))
}

// ── GET /new-releases ────────────────────────────────────────────────────

async fn new_releases(
    claims: Claims,
    State(state): State<AppState>,
) -> AppResult<Json<Vec<NewReleaseDto>>> {
    let items = titles::new_releases(&state.db, &claims.user_id, claims.allow_explicit).await?;
    Ok(Json(items.into_iter().map(NewReleaseDto::from).collect()))
}

// ── GET /{id} ────────────────────────────────────────────────────────────

async fn get_title(
    claims: Claims,
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> AppResult<Json<TitleDto>> {
    let t = titles::get_title(&state.db, &id, &claims.user_id, claims.allow_explicit)
        .await?
        .ok_or(AppError::NotFound)?;
    Ok(Json(TitleDto::from(t)))
}

// ── DELETE /{id} ─────────────────────────────────────────────────────────

#[derive(Deserialize)]
struct RemoveQuery {
    delete_files: Option<bool>,
}

async fn remove_title(
    claims: Claims,
    State(state): State<AppState>,
    Path(id): Path<String>,
    Query(q): Query<RemoveQuery>,
) -> AppResult<StatusCode> {
    let is_admin = claims.role == "admin";
    if !titles::remove_user_title(&state.db, &claims.user_id, &id).await? {
        return Err(AppError::NotFound);
    }

    if titles::count_owners(&state.db, &id).await? == 0 {
        if q.delete_files.unwrap_or(false) && is_admin {
            let paths = titles::chapter_local_paths(&state.db, &id).await?;
            for p in paths.iter().filter(|p| !p.starts_with("http")) {
                let _ = std::fs::remove_file(p);
            }
            if let Some(cover) = titles::cover_url(&state.db, &id).await? {
                if !cover.starts_with("http") {
                    let _ = std::fs::remove_file(&cover);
                }
            }
        }
        titles::delete_title(&state.db, &id).await?;
    }

    Ok(StatusCode::NO_CONTENT)
}

// ── PATCH /{id} ──────────────────────────────────────────────────────────

const VALID_CONTENT_TYPES: [&str; 4] = ["manga", "manhwa", "manhua", "novel"];

#[derive(Deserialize)]
struct PatchBody {
    auto_download: Option<bool>,
    reader_mode: Option<String>,
    download_dir: Option<String>,
    is_explicit: Option<bool>,
    cover_url: Option<String>,
    content_type: Option<String>,
}

async fn patch_title(
    claims: Claims,
    State(state): State<AppState>,
    Path(id): Path<String>,
    Json(body): Json<PatchBody>,
) -> AppResult<StatusCode> {
    let is_admin = claims.role == "admin";
    if !titles::is_owned(&state.db, &claims.user_id, &id).await? {
        return Err(AppError::NotFound);
    }

    if let Some(v) = body.auto_download {
        titles::update_auto_download(&state.db, &id, v).await?;
    }

    if let Some(rm) = &body.reader_mode {
        if rm != "paged" && rm != "scroll" {
            return Err(AppError::UnprocessableEntity("invalid reader_mode".into()));
        }
        titles::upsert_reader_mode(&state.db, &claims.user_id, &id, rm).await?;
    }

    if let Some(dir) = &body.download_dir {
        titles::update_download_dir(&state.db, &id, Some(dir)).await?;
    }

    if let Some(v) = body.is_explicit {
        if !is_admin {
            return Err(AppError::Forbidden);
        }
        titles::update_is_explicit(&state.db, &id, v).await?;
    }

    if let Some(cover) = &body.cover_url {
        if !is_admin {
            return Err(AppError::Forbidden);
        }
        let v = if cover.is_empty() {
            None
        } else {
            Some(cover.as_str())
        };
        titles::update_cover_url(&state.db, &id, v).await?;
    }

    if let Some(ct) = &body.content_type {
        if !is_admin {
            return Err(AppError::Forbidden);
        }
        if !VALID_CONTENT_TYPES.contains(&ct.as_str()) {
            return Err(AppError::UnprocessableEntity("invalid content_type".into()));
        }
        let current = titles::get_content_type(&state.db, &id)
            .await?
            .ok_or(AppError::NotFound)?;
        if &current != ct {
            titles::update_content_type(&state.db, &id, ct).await?;
            // ReMatchSourcesAsync stub — see module doc.
            titles::update_sync_status(&state.db, &id, "ready").await?;
        }
    }

    Ok(StatusCode::NO_CONTENT)
}

// ── POST /{id}/sync ──────────────────────────────────────────────────────

async fn sync_title(
    claims: Claims,
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> AppResult<StatusCode> {
    let content_type = titles::owned_content_type(&state.db, &claims.user_id, &id)
        .await?
        .ok_or(AppError::NotFound)?;
    let links = titles::title_source_links(&state.db, &id).await?;
    if links.is_empty() {
        return Err(AppError::NotFound);
    }

    titles::update_sync_status(&state.db, &id, "syncing").await?;
    titles::clear_sync_log(&state.db, &id).await?;

    let db = state.db.clone();
    let http = state.http.clone();
    let plugin_host_url = state.config.plugin_host_url.clone();
    tokio::spawn(async move {
        let mut any_error = false;
        for (source, source_id) in &links {
            titles::append_sync_log(&db, &id, &format!("Syncing from {source}…")).await;
            match chapters::sync_from_source(
                &db,
                &http,
                &plugin_host_url,
                &id,
                &content_type,
                source,
                source_id,
            )
            .await
            {
                Ok(count) => {
                    titles::append_sync_log(
                        &db,
                        &id,
                        &format!("Synced {count} chapter(s) from {source}"),
                    )
                    .await
                }
                Err(e) => {
                    any_error = true;
                    titles::append_sync_log(&db, &id, &format!("Error from {source}: {e}")).await;
                }
            }
        }
        let final_status = if any_error { "error" } else { "ready" };
        let _ = titles::update_sync_status(&db, &id, final_status).await;
        titles::append_sync_log(&db, &id, "Sync complete").await;
    });

    Ok(StatusCode::ACCEPTED)
}

// ── GET /{id}/sync-log ───────────────────────────────────────────────────

async fn get_sync_log(
    claims: Claims,
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> AppResult<Json<Vec<titles::SyncLogEntry>>> {
    if !titles::is_owned(&state.db, &claims.user_id, &id).await? {
        return Err(AppError::NotFound);
    }
    Ok(Json(titles::list_sync_log(&state.db, &id).await?))
}

// ── POST /{id}/refresh-metadata ──────────────────────────────────────────

async fn refresh_metadata(
    claims: Claims,
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> AppResult<StatusCode> {
    if !titles::is_owned(&state.db, &claims.user_id, &id).await? {
        return Err(AppError::NotFound);
    }

    titles::clear_sync_warnings(&state.db, &id).await?;
    titles::clear_sync_log(&state.db, &id).await?;

    // MU metadata + alias refresh — only for MU-sourced titles (S6 #128).
    if let Some(mu_id_str) = titles::get_mangaupdates_id(&state.db, &id).await? {
        if let Ok(mu_id) = mu_id_str.parse::<u64>() {
            if let Ok(Some(series)) = crate::metadata::mangaupdates::series_detail(
                &state.http,
                &state.config.mangaupdates_url,
                mu_id,
            )
            .await
            {
                if let Some(cover) = &series.cover_url {
                    titles::set_cover_url_if_absent(&state.db, &id, cover).await?;
                }
                if !series.associated_names.is_empty() {
                    titles::clear_title_aliases(&state.db, &id).await?;
                    for alias in &series.associated_names {
                        titles::insert_title_alias(&state.db, &id, alias).await?;
                    }
                }
            }
        }
    }

    titles::update_sync_status(&state.db, &id, "syncing").await?;

    let db = state.db.clone();
    let http = state.http.clone();
    let plugin_host_url = state.config.plugin_host_url.clone();
    tokio::spawn(async move {
        if let Some(t) = titles::fetch_title(&db, &id, "").await.ok().flatten() {
            crate::discover::match_sources(
                &db,
                &http,
                &plugin_host_url,
                &id,
                &t.title,
                &t.content_type,
                t.is_explicit,
            )
            .await;
        }
        let _ = titles::update_sync_status(&db, &id, "ready").await;
    });

    Ok(StatusCode::ACCEPTED)
}
