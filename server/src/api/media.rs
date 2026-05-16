use axum::{
    extract::{Path, Query, State},
    http::{header, StatusCode},
    response::{IntoResponse, Response},
    routing::get,
    Router,
};
use serde::Deserialize;
use tokio::fs;

use crate::{media::{get_chapter_page, strip_jpeg_icc}, AppState};
use super::ApiResult;

#[derive(Deserialize)]
struct ProxyQuery {
    url: String,
}

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/media/page/{chapter_id}/{page}", get(serve_page))
        .route("/media/cover/{manga_id}", get(serve_cover))
        .route("/media/proxy", get(proxy_image))
}

async fn proxy_image(
    State(state): State<AppState>,
    Query(params): Query<ProxyQuery>,
) -> ApiResult<Response> {
    let Some(src) = state.sources.get("mangapill") else {
        return Ok(StatusCode::BAD_GATEWAY.into_response());
    };
    let bytes = match src.fetch_cover(&params.url).await {
        Ok(b) => b,
        Err(_) => return Ok(StatusCode::BAD_GATEWAY.into_response()),
    };

    let ct = if params.url.contains(".webp") { "image/webp" }
             else if params.url.contains(".png") { "image/png" }
             else { "image/jpeg" };

    Ok((
        [(header::CONTENT_TYPE, ct)],
        bytes,
    ).into_response())
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
        let Some(path) = chapter.local_path else {
            return Ok(StatusCode::NOT_FOUND.into_response());
        };
        let data = match get_chapter_page(&path, page as usize).await {
            Ok(d) => d,
            Err(_) => return Ok(StatusCode::NOT_FOUND.into_response()),
        };
        return Ok(([(header::CONTENT_TYPE, "image/jpeg")], data).into_response());
    }

    let Some(source_id) = chapter.source_id else {
        return Ok(StatusCode::NOT_FOUND.into_response());
    };
    let Some(src) = state.sources.get(&chapter.source) else {
        return Ok(StatusCode::BAD_GATEWAY.into_response());
    };
    let url = match src.get_page_url(&source_id, page as usize).await {
        Ok(u) => u,
        Err(_) => return Ok(StatusCode::BAD_GATEWAY.into_response()),
    };

    let client = reqwest::Client::new();
    let resp = match client
        .get(&url)
        .header("User-Agent", "Mozilla/5.0")
        .header("Referer", "https://mangadex.org/")
        .send()
        .await
    {
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
