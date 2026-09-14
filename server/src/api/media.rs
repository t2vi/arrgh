//! `/api/media` — port of `Api/Media.cs` (ADR 0033, S8 #130). No auth on
//! any route, matching .NET — see `crate::media`'s module doc.

use axum::extract::{Path, Query, State};
use axum::http::{header, StatusCode};
use axum::response::{IntoResponse, Redirect, Response};

use crate::error::{AppError, AppResult};
use crate::media;
use crate::state::AppState;

pub fn routes() -> axum::Router<AppState> {
    axum::Router::new()
        .route("/page/{chapter_id}/{page}", axum::routing::get(serve_page))
        .route("/cover/{title_id}", axum::routing::get(serve_cover))
        .route("/meta-cover", axum::routing::get(serve_meta_cover))
        .route("/proxy", axum::routing::get(proxy_image))
}

fn bytes_response(content_type: &str, data: Vec<u8>) -> Response {
    (
        StatusCode::OK,
        [(header::CONTENT_TYPE, content_type.to_string())],
        data,
    )
        .into_response()
}

// ── GET /api/media/page/{chapter_id}/{page} ─────────────────────────────

async fn serve_page(
    State(state): State<AppState>,
    Path((chapter_id, page)): Path<(String, i64)>,
) -> AppResult<Response> {
    let page = page.max(0) as usize;

    let info = media::chapter_local_info(&state.db, &chapter_id)
        .await?
        .ok_or(AppError::NotFound)?;

    if info.downloaded {
        if let Some(path) = &info.local_path {
            if let Some(data) = media::get_chapter_page(path, page).await {
                if let Some(ct) = media::detect_content_type(&data) {
                    return Ok(bytes_response(ct, media::strip_jpeg_icc(&data)));
                }
            }
        }
        media::reset_chapter_local(&state.db, &chapter_id).await?;
    }

    let links = media::chapter_source_links(&state.db, &chapter_id).await?;
    if links.is_empty() {
        return Err(AppError::NotFound);
    }

    let pages = match state.page_cache.get(&chapter_id) {
        Some(cached) => cached,
        None => {
            let mut fetched = None;
            for (source_id, base_url) in links
                .iter()
                .filter_map(|(sid, base)| base.as_ref().map(|b| (sid, b)))
            {
                let url = format!(
                    "{}/chapter/{}/pages",
                    base_url.trim_end_matches('/'),
                    urlencoding::encode(source_id)
                );
                let Ok(res) = state.http.get(&url).send().await else {
                    continue;
                };
                if !res.status().is_success() {
                    continue;
                }
                let Ok(bytes) = res.bytes().await else {
                    continue;
                };
                if let Ok(parsed) = crate::downloader::parse_page_urls(&bytes) {
                    fetched = Some(parsed);
                    break;
                }
            }
            let Some(fetched) = fetched else {
                return Err(AppError::BadGateway);
            };
            state.page_cache.set(&chapter_id, fetched.clone());
            fetched
        }
    };

    let Some((page_url, referer)) = pages.get(page) else {
        return Err(AppError::NotFound);
    };

    let mut req = state.http.get(page_url).header("User-Agent", "Mozilla/5.0");
    if let Some(r) = referer {
        req = req.header("Referer", r.as_str());
    }
    let Ok(res) = req.send().await else {
        return Err(AppError::BadGateway);
    };
    let content_type = res
        .headers()
        .get(header::CONTENT_TYPE)
        .and_then(|v| v.to_str().ok())
        .unwrap_or("image/jpeg")
        .to_string();
    let bytes = res.bytes().await?;
    Ok(bytes_response(&content_type, media::strip_jpeg_icc(&bytes)))
}

// ── GET /api/media/cover/{title_id} ─────────────────────────────────────

async fn serve_cover(
    State(state): State<AppState>,
    Path(title_id): Path<String>,
) -> AppResult<Response> {
    let (cover_url, title_name) = media::title_cover_info(&state.db, &title_id)
        .await?
        .ok_or(AppError::NotFound)?;

    if let Some(url) = cover_url {
        if url.starts_with("http") || url.starts_with("/api/") {
            return Ok(Redirect::to(&url).into_response());
        }
        match tokio::fs::read(&url).await {
            Ok(data) => {
                return Ok(bytes_response(media::content_type_from_path(&url), data));
            }
            Err(_) => {
                media::clear_title_cover(&state.db, &title_id).await?;
            }
        }
    }

    let key = crate::discover::normalize_title(&title_name);
    if let Some(cdn_url) = media::title_meta_cover_cdn(&state.db, &key).await? {
        media::set_title_cover(&state.db, &title_id, &cdn_url).await?;
        return Ok(Redirect::to(&cdn_url).into_response());
    }

    Err(AppError::NotFound)
}

// ── GET /api/media/meta-cover?key= ──────────────────────────────────────

#[derive(serde::Deserialize)]
struct MetaCoverQuery {
    key: String,
}

async fn serve_meta_cover(
    State(state): State<AppState>,
    Query(q): Query<MetaCoverQuery>,
) -> AppResult<Response> {
    let (cover_local_path, cover_cdn_url) = media::title_meta_cover_row(&state.db, &q.key)
        .await?
        .ok_or(AppError::NotFound)?;

    if let Some(path) = cover_local_path {
        if let Ok(data) = tokio::fs::read(&path).await {
            if media::detect_content_type(&data).is_some() {
                return Ok(bytes_response(media::content_type_from_path(&path), data));
            }
        }
        media::clear_title_meta_local_path(&state.db, &q.key).await?;
    }

    if let Some(cdn) = cover_cdn_url {
        let proxied = format!("/api/media/proxy?url={}", urlencoding::encode(&cdn));
        return Ok(Redirect::to(&proxied).into_response());
    }

    Err(AppError::NotFound)
}

// ── GET /api/media/proxy?url= ────────────────────────────────────────────

#[derive(serde::Deserialize)]
struct ProxyQuery {
    url: String,
}

async fn proxy_image(
    State(state): State<AppState>,
    Query(q): Query<ProxyQuery>,
) -> AppResult<Response> {
    let referer = media::root_domain_referer(&q.url);
    let mut req = state.http.get(&q.url).header("User-Agent", "Mozilla/5.0");
    if !referer.is_empty() {
        req = req.header("Referer", referer);
    }

    let Ok(res) = req.send().await else {
        return Err(AppError::BadGateway);
    };
    if !res.status().is_success() {
        return Err(AppError::BadGateway);
    }

    let bytes = res.bytes().await?;
    let ct = if q.url.contains(".webp") {
        "image/webp"
    } else if q.url.contains(".png") {
        "image/png"
    } else {
        "image/jpeg"
    };
    Ok(bytes_response(ct, bytes.to_vec()))
}
