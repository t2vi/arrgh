use axum::{
    extract::{Path, Query, State},
    http::{header, StatusCode},
    response::{IntoResponse, Response},
    routing::get,
    Router,
};
use serde::Deserialize;
use std::sync::Arc;
use tokio::fs;

use crate::{media::{get_chapter_page, strip_jpeg_icc}, AppState};
use super::ApiResult;

#[derive(Deserialize)]
struct ProxyQuery {
    url: String,
}

#[derive(Deserialize)]
struct MetaCoverQuery {
    key: String,
}

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/media/page/{chapter_id}/{page}", get(serve_page))
        .route("/media/cover/{manga_id}", get(serve_cover))
        .route("/media/meta-cover", get(serve_meta_cover))
        .route("/media/proxy", get(proxy_image))
}

fn root_domain_referer(url: &str) -> String {
    if let Ok(parsed) = reqwest::Url::parse(url) {
        let host = parsed.host_str().unwrap_or("");
        let parts: Vec<&str> = host.split('.').collect();
        let root = if parts.len() >= 2 {
            format!("{}.{}", parts[parts.len() - 2], parts[parts.len() - 1])
        } else {
            host.to_string()
        };
        format!("{}://{}", parsed.scheme(), root)
    } else {
        String::new()
    }
}

async fn proxy_image(
    Query(params): Query<ProxyQuery>,
) -> ApiResult<Response> {
    let referer = root_domain_referer(&params.url);
    let client = reqwest::Client::new();
    let bytes = match client
        .get(&params.url)
        .header("User-Agent", "Mozilla/5.0")
        .header("Referer", &referer)
        .send()
        .await
        .and_then(|r| r.error_for_status())
    {
        Ok(r) => match r.bytes().await {
            Ok(b) => b.to_vec(),
            Err(_) => return Ok(StatusCode::BAD_GATEWAY.into_response()),
        },
        Err(_) => return Ok(StatusCode::BAD_GATEWAY.into_response()),
    };

    let ct = if params.url.contains(".webp") { "image/webp" }
             else if params.url.contains(".png") { "image/png" }
             else { "image/jpeg" };

    Ok(([(header::CONTENT_TYPE, ct)], bytes).into_response())
}

fn image_content_type(data: &[u8]) -> Option<&'static str> {
    if data.len() < 4 { return None; }
    if data[0] == 0xFF && data[1] == 0xD8 { return Some("image/jpeg"); }
    if data.starts_with(b"\x89PNG") { return Some("image/png"); }
    if data.len() >= 12 && data.starts_with(b"RIFF") && &data[8..12] == b"WEBP" { return Some("image/webp"); }
    if data.starts_with(b"GIF8") { return Some("image/gif"); }
    if data.len() >= 8 && &data[4..8] == b"ftyp" { return Some("image/avif"); }
    None
}

async fn serve_page(
    State(state): State<AppState>,
    Path((chapter_id, page)): Path<(String, u32)>,
) -> ApiResult<Response> {
    let chapter = sqlx::query!(
        r#"SELECT c.local_path, c.downloaded, c.source_id, m.source as "source!"
           FROM chapters c JOIN manga m ON c.manga_id = m.id
           WHERE c.id = ?"#,
        chapter_id
    )
    .fetch_optional(&state.db)
    .await?;

    let Some(chapter) = chapter else {
        return Ok(StatusCode::NOT_FOUND.into_response());
    };

    if chapter.downloaded != 0 {
        let file_ok = match &chapter.local_path {
            Some(path) => get_chapter_page(path, page as usize).await.ok(),
            None => None,
        };
        if let Some(data) = file_ok {
            if let Some(ct) = image_content_type(&data) {
                let data = strip_jpeg_icc(data);
                return Ok(([(header::CONTENT_TYPE, ct)], data).into_response());
            }
            // Non-image bytes (e.g. CF challenge HTML stored in old download) — reset and stream
            tracing::warn!("chapter {} page {} has corrupt data, resetting downloaded flag", chapter_id, page);
        } else {
            tracing::warn!("chapter {} marked downloaded but files missing, resetting", chapter_id);
        }
        let _ = sqlx::query!(
            "UPDATE chapters SET downloaded = 0, local_path = NULL WHERE id = ?",
            chapter_id
        )
        .execute(&state.db)
        .await;
    }

    let Some(source_id) = chapter.source_id else {
        return Ok(StatusCode::NOT_FOUND.into_response());
    };
    let src = state.sources.read().await.get(&chapter.source).cloned();
    let Some(src) = src else {
        return Ok(StatusCode::BAD_GATEWAY.into_response());
    };

    // Cache page URLs so 20 concurrent scroll requests don't each trigger a FlareSolverr call
    let cache_key = format!("{}/{}", chapter.source, source_id);
    let cached = {
        let cache = state.page_cache.lock().await;
        cache.get(&cache_key).and_then(|(ts, urls)| {
            if ts.elapsed() < std::time::Duration::from_secs(300) {
                Some(Arc::clone(urls))
            } else {
                None
            }
        })
    };

    let pages = if let Some(cached) = cached {
        cached
    } else {
        let urls = match src.get_page_urls(&source_id).await {
            Ok(p) => p,
            Err(e) => {
                tracing::debug!("get_page_urls failed for {}: {}", source_id, e);
                return Ok(StatusCode::BAD_GATEWAY.into_response());
            }
        };
        let arc = Arc::new(urls);
        let mut cache = state.page_cache.lock().await;
        if cache.len() > 200 {
            cache.retain(|_, (ts, _)| ts.elapsed() < std::time::Duration::from_secs(300));
        }
        cache.insert(cache_key, (std::time::Instant::now(), Arc::clone(&arc)));
        arc
    };

    let Some(page_url) = pages.get(page as usize) else {
        return Ok(StatusCode::NOT_FOUND.into_response());
    };

    let client = reqwest::Client::new();
    let mut req = client.get(&page_url.url).header("User-Agent", "Mozilla/5.0");
    if let Some(ref referer) = page_url.referer {
        req = req.header("Referer", referer);
    }
    let resp = match req.send().await {
        Ok(r) => r,
        Err(_) => return Ok(StatusCode::BAD_GATEWAY.into_response()),
    };

    let ct = resp
        .headers()
        .get(reqwest::header::CONTENT_TYPE)
        .and_then(|v| v.to_str().ok())
        .unwrap_or("image/jpeg")
        .to_string();

    let bytes = match resp.bytes().await {
        Ok(b) => b.to_vec(),
        Err(_) => return Ok(StatusCode::BAD_GATEWAY.into_response()),
    };

    let data = strip_jpeg_icc(bytes);

    Ok(([(header::CONTENT_TYPE, ct)], data).into_response())
}

async fn serve_meta_cover(
    State(state): State<AppState>,
    Query(params): Query<MetaCoverQuery>,
) -> ApiResult<Response> {
    let row = sqlx::query!(
        "SELECT cover_local_path, cover_cdn_url FROM title_meta WHERE title_key = ?",
        params.key
    )
    .fetch_optional(&state.db)
    .await?;

    let Some(row) = row else {
        return Ok(StatusCode::NOT_FOUND.into_response());
    };

    if let Some(ref path) = row.cover_local_path {
        if let Ok(data) = fs::read(path).await {
            if image_content_type(&data).is_some() {
                let ct = if path.ends_with(".webp") { "image/webp" }
                         else if path.ends_with(".png") { "image/png" }
                         else { "image/jpeg" };
                return Ok(([(header::CONTENT_TYPE, ct)], data).into_response());
            }
            // Corrupt file — clear local path so next seed re-downloads
            tracing::warn!("meta cover for '{}' is corrupt ({} bytes), clearing", params.key, data.len());
            let _ = sqlx::query!(
                "UPDATE title_meta SET cover_local_path = NULL WHERE title_key = ?",
                params.key
            )
            .execute(&state.db)
            .await;
        }
    }

    // Fall back to CDN via proxy
    if let Some(cdn_url) = row.cover_cdn_url {
        let proxy_url = format!("/api/media/proxy?url={}", urlencoding::encode(&cdn_url));
        return Ok(axum::response::Redirect::temporary(&proxy_url).into_response());
    }

    Ok(StatusCode::NOT_FOUND.into_response())
}

async fn serve_cover(
    State(state): State<AppState>,
    Path(manga_id): Path<String>,
) -> ApiResult<Response> {
    let manga = sqlx::query!("SELECT cover_url FROM manga WHERE id = ?", manga_id)
        .fetch_optional(&state.db)
        .await?;

    let Some(manga) = manga else {
        return Ok(StatusCode::NOT_FOUND.into_response());
    };
    let Some(cover_path) = manga.cover_url else {
        return Ok(StatusCode::NOT_FOUND.into_response());
    };

    if cover_path.starts_with("http") {
        return Ok(axum::response::Redirect::temporary(&cover_path).into_response());
    }

    let data = match fs::read(&cover_path).await {
        Ok(d) => d,
        Err(_) => return Ok(StatusCode::NOT_FOUND.into_response()),
    };

    let ct = if cover_path.ends_with(".webp") { "image/webp" }
             else if cover_path.ends_with(".png") { "image/png" }
             else { "image/jpeg" };

    Ok(([(header::CONTENT_TYPE, ct)], data).into_response())
}
