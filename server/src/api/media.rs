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
        .route("/media/cover/{title_id}", get(serve_cover))
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

#[cfg(test)]
mod tests {
    use super::*;

    // ── image_content_type ────────────────────────────────────────────────────

    #[test]
    fn jpeg_magic_bytes() {
        assert_eq!(image_content_type(&[0xFF, 0xD8, 0x00, 0x00]), Some("image/jpeg"));
    }

    #[test]
    fn png_magic_bytes() {
        assert_eq!(image_content_type(b"\x89PNG\r\n\x1a\n"), Some("image/png"));
    }

    #[test]
    fn webp_magic_bytes() {
        let mut data = *b"RIFF\x00\x00\x00\x00WEBP";
        data[4..8].copy_from_slice(&[0, 0, 0, 0]);
        assert_eq!(image_content_type(&data), Some("image/webp"));
    }

    #[test]
    fn gif_magic_bytes() {
        assert_eq!(image_content_type(b"GIF89a\x00\x00"), Some("image/gif"));
    }

    #[test]
    fn too_short_returns_none() {
        assert_eq!(image_content_type(&[0xFF, 0xD8, 0x00]), None);
        assert_eq!(image_content_type(&[]), None);
    }

    #[test]
    fn unknown_bytes_returns_none() {
        assert_eq!(image_content_type(&[0x00, 0x01, 0x02, 0x03]), None);
    }

    // ── root_domain_referer ───────────────────────────────────────────────────

    #[test]
    fn extracts_root_domain() {
        assert_eq!(root_domain_referer("https://cdn.mangapill.com/img/page.jpg"), "https://mangapill.com");
    }

    #[test]
    fn handles_apex_domain() {
        assert_eq!(root_domain_referer("https://mangadex.org/chapter/abc"), "https://mangadex.org");
    }

    #[test]
    fn invalid_url_returns_empty() {
        assert_eq!(root_domain_referer("not-a-url"), "");
        assert_eq!(root_domain_referer(""), "");
    }

    #[test]
    fn preserves_scheme() {
        assert_eq!(root_domain_referer("http://sub.example.com/path"), "http://example.com");
    }
}

async fn serve_page(
    State(state): State<AppState>,
    Path((chapter_id, page)): Path<(String, u32)>,
) -> ApiResult<Response> {
    let chapter = sqlx::query!(
        r#"SELECT c.local_path, c.downloaded FROM chapters c
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

    // Load source links ordered by priority — use the cache key from the first successful source
    let source_links = sqlx::query!(
        r#"SELECT cs.source as "source!", cs.source_id as "source_id!"
           FROM chapter_sources cs
           LEFT JOIN external_sources es ON es.source_key = cs.source
           WHERE cs.chapter_id = ?
           ORDER BY COALESCE(es.priority, 100) ASC"#,
        chapter_id
    )
    .fetch_all(&state.db)
    .await?;

    if source_links.is_empty() {
        return Ok(StatusCode::NOT_FOUND.into_response());
    }

    // Try cached pages first
    let cache_key = chapter_id.clone();
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
        let mut fetched = None;
        for link in &source_links {
            let src = state.sources.read().await.get(&link.source).cloned();
            let Some(src) = src else { continue; };
            match src.get_page_urls(&link.source_id).await {
                Ok(urls) => { fetched = Some(urls); break; }
                Err(e) => tracing::debug!("get_page_urls failed for {} ({}): {}", chapter_id, link.source, e),
            }
        }
        let Some(urls) = fetched else {
            return Ok(StatusCode::BAD_GATEWAY.into_response());
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
        match fs::read(path).await {
            Ok(data) if image_content_type(&data).is_some() => {
                let ct = if path.ends_with(".webp") { "image/webp" }
                         else if path.ends_with(".png") { "image/png" }
                         else { "image/jpeg" };
                return Ok(([(header::CONTENT_TYPE, ct)], data).into_response());
            }
            Ok(data) => {
                tracing::warn!("meta cover for '{}' is corrupt ({} bytes), clearing", params.key, data.len());
                let _ = sqlx::query!(
                    "UPDATE title_meta SET cover_local_path = NULL WHERE title_key = ?",
                    params.key
                )
                .execute(&state.db)
                .await;
            }
            Err(_) => {
                // File missing — clear so next discover request re-downloads it
                let _ = sqlx::query!(
                    "UPDATE title_meta SET cover_local_path = NULL WHERE title_key = ?",
                    params.key
                )
                .execute(&state.db)
                .await;
            }
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
    Path(title_id): Path<String>,
) -> ApiResult<Response> {
    let manga = sqlx::query!("SELECT cover_url, title FROM titles WHERE id = ?", title_id)
        .fetch_optional(&state.db)
        .await?;

    let Some(manga) = manga else {
        return Ok(StatusCode::NOT_FOUND.into_response());
    };

    if let Some(ref cover_path) = manga.cover_url {
        if cover_path.starts_with("http") {
            return Ok(axum::response::Redirect::temporary(cover_path).into_response());
        }
        // Bad DB entry — internal meta-cover path stored instead of CDN URL; redirect.
        if cover_path.starts_with("/api/") {
            return Ok(axum::response::Redirect::temporary(cover_path).into_response());
        }

        match fs::read(cover_path).await {
            Ok(d) => {
                let ct = if cover_path.ends_with(".webp") { "image/webp" }
                         else if cover_path.ends_with(".png") { "image/png" }
                         else { "image/jpeg" };
                return Ok(([(header::CONTENT_TYPE, ct)], d).into_response());
            }
            Err(_) => {
                // Local file missing (e.g. container rebuilt without persisting the path).
                // Null it out and fall through to CDN fallback.
                let _ = sqlx::query!(
                    "UPDATE titles SET cover_url = NULL WHERE id = ?",
                    title_id
                )
                .execute(&state.db)
                .await;
            }
        }
    }

    // CDN fallback: look up cached URL in title_meta and redirect there.
    // This recovers thumbnails after a rebuild wipes non-volume paths.
    let key = crate::api::discover::normalize_title(&manga.title);
    let cdn = sqlx::query_scalar!(
        "SELECT cover_cdn_url FROM title_meta WHERE title_key = ?", key
    )
    .fetch_optional(&state.db)
    .await?
    .flatten();

    if let Some(cdn_url) = cdn {
        // Restore so the next request skips this lookup entirely.
        let _ = sqlx::query!(
            "UPDATE titles SET cover_url = ? WHERE id = ?",
            cdn_url, title_id
        )
        .execute(&state.db)
        .await;
        return Ok(axum::response::Redirect::temporary(&cdn_url).into_response());
    }

    Ok(StatusCode::NOT_FOUND.into_response())
}
