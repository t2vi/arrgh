//! S8 parity check for `/api/media` (ADR 0033, S8 #130) — page/cover
//! serving. Pure helpers (content-type sniffing, ICC stripping, archive/dir
//! page selection) are covered by `src/media.rs`'s own unit tests, not
//! repeated here; this file proves the HTTP-layer contract: DB lookup →
//! local file → source fallback → not-found, and cover redirect/file/
//! fallback/not-found.

mod common;

use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

use axum::body::Body;
use axum::http::{HeaderMap, Request, StatusCode};
use axum::Router;
use http_body_util::BodyExt;
use tower::ServiceExt; // oneshot

async fn send(app: &Router, uri: &str) -> (StatusCode, HeaderMap, Vec<u8>) {
    let req = Request::builder()
        .method("GET")
        .uri(uri)
        .body(Body::empty())
        .unwrap();
    let res = app.clone().oneshot(req).await.unwrap();
    let status = res.status();
    let headers = res.headers().clone();
    let bytes = res.into_body().collect().await.unwrap().to_bytes().to_vec();
    (status, headers, bytes)
}

/// A self-referencing fake source: `/chapter/{source_id}/pages` returns a
/// one-page JSON array pointing back at this same server's `/img/0.jpg`,
/// which serves `body`. `hits` counts pages-list requests, to prove
/// `state.page_cache` is reused rather than re-resolved per page request
/// (FR-004).
async fn start_source_mock(body: &'static str, hits: Arc<AtomicUsize>) -> String {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    let base = format!("http://{addr}");
    let base_for_pages = base.clone();
    let app = axum::Router::new()
        .route(
            "/chapter/{source_id}/pages",
            axum::routing::get(move || {
                let base = base_for_pages.clone();
                let hits = hits.clone();
                async move {
                    hits.fetch_add(1, Ordering::SeqCst);
                    axum::Json(vec![format!("{base}/img/0.jpg")])
                }
            }),
        )
        .route(
            "/img/0.jpg",
            axum::routing::get(move || async move { body }),
        );
    tokio::spawn(async move {
        axum::serve(listener, app).await.unwrap();
    });
    base
}

// ── GET /api/media/page/{chapter_id}/{page} ─────────────────────────────

// spec: 006/FR-001, 006/FR-002, 006/FR-012
#[tokio::test]
async fn serve_page_from_downloaded_directory_returns_bytes_with_sniffed_content_type() {
    let state = common::build_state().await;
    let t = common::seed_title(&state, "Naruto", false).await;
    let c = common::seed_chapter(&state, &t, 1.0, true).await;

    let dir = std::env::temp_dir().join(format!("arrgh-media-page-{}", uuid::Uuid::new_v4()));
    tokio::fs::create_dir_all(&dir).await.unwrap();
    tokio::fs::write(
        dir.join("01.png"),
        [0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A],
    )
    .await
    .unwrap();
    sqlx::query("UPDATE chapters SET local_path = ? WHERE id = ?")
        .bind(dir.to_str().unwrap())
        .bind(&c)
        .execute(&state.db)
        .await
        .unwrap();

    let app = arrgh_server::api::router(state);
    // No Authorization header at all — proves FR-012 (unauthenticated).
    let (status, headers, bytes) = send(&app, &format!("/api/media/page/{c}/0")).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(headers.get("content-type").unwrap(), "image/png");
    assert_eq!(bytes, vec![0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A]);
}

// spec: 006/FR-003
#[tokio::test]
async fn serve_page_falls_back_to_source_and_clears_stale_local_state() {
    let state = common::build_state().await;
    let t = common::seed_title(&state, "Naruto", false).await;
    let c = common::seed_chapter(&state, &t, 1.0, true).await; // downloaded=true
    sqlx::query("UPDATE chapters SET local_path = '/nonexistent/path' WHERE id = ?")
        .bind(&c)
        .execute(&state.db)
        .await
        .unwrap();

    let mock = start_source_mock("proxied-bytes", Arc::new(AtomicUsize::new(0))).await;
    sqlx::query(
        "INSERT INTO external_sources (id, name, base_url, content_types, enabled, created_at, is_community, priority, source_key, default_explicit) \
         VALUES (?, 'mock', ?, 'manga', 1, datetime('now'), 0, 1, 'mock', 0)",
    )
    .bind(uuid::Uuid::new_v4().to_string())
    .bind(&mock)
    .execute(&state.db)
    .await
    .unwrap();
    common::add_chapter_source(&state, &c, "mock").await;

    let app = arrgh_server::api::router(state.clone());
    let (status, _, bytes) = send(&app, &format!("/api/media/page/{c}/0")).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(bytes, b"proxied-bytes");

    let (downloaded, local_path): (bool, Option<String>) =
        sqlx::query_as("SELECT downloaded, local_path FROM chapters WHERE id = ?")
            .bind(&c)
            .fetch_one(&state.db)
            .await
            .unwrap();
    assert!(!downloaded);
    assert_eq!(local_path, None);
}

// spec: 006/FR-004
#[tokio::test]
async fn serve_page_tries_lower_priority_source_first_and_caches_resolved_list() {
    let state = common::build_state().await;
    let t = common::seed_title(&state, "Naruto", false).await;
    let c = common::seed_chapter(&state, &t, 1.0, false).await;

    let failing = common::start_mock_routes(&[("/chapter/x/pages", 500, "boom")]).await;
    let succeeding_hits = Arc::new(AtomicUsize::new(0));
    let succeeding = start_source_mock("page-bytes", succeeding_hits.clone()).await;

    for (base, key, priority) in [(&failing, "bad", 1i64), (&succeeding, "good", 2i64)] {
        sqlx::query(
            "INSERT INTO external_sources (id, name, base_url, content_types, enabled, created_at, is_community, priority, source_key, default_explicit) \
             VALUES (?, ?, ?, 'manga', 1, datetime('now'), 0, ?, ?, 0)",
        )
        .bind(uuid::Uuid::new_v4().to_string())
        .bind(key)
        .bind(base)
        .bind(priority)
        .bind(key)
        .execute(&state.db)
        .await
        .unwrap();
        common::add_chapter_source(&state, &c, key).await;
    }

    let app = arrgh_server::api::router(state);
    let (status, _, bytes) = send(&app, &format!("/api/media/page/{c}/0")).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(bytes, b"page-bytes");
    assert_eq!(succeeding_hits.load(Ordering::SeqCst), 1);

    // A second page request for the same chapter must reuse the cached
    // resolved list rather than re-hitting the pages endpoint.
    send(&app, &format!("/api/media/page/{c}/0")).await;
    assert_eq!(succeeding_hits.load(Ordering::SeqCst), 1);
}

// spec: 006/FR-005
#[tokio::test]
async fn serve_page_not_found_when_no_local_copy_and_no_linked_sources() {
    let state = common::build_state().await;
    let t = common::seed_title(&state, "Naruto", false).await;
    let c = common::seed_chapter(&state, &t, 1.0, false).await;

    let app = arrgh_server::api::router(state);
    let (status, _, _) = send(&app, &format!("/api/media/page/{c}/0")).await;
    assert_eq!(status, StatusCode::NOT_FOUND);
}

// ── GET /api/media/cover/{title_id} ──────────────────────────────────────

// spec: 006/FR-007, 006/FR-012
#[tokio::test]
async fn serve_cover_redirects_when_cover_url_is_remote() {
    let state = common::build_state().await;
    let t = common::seed_title(&state, "Naruto", false).await;
    sqlx::query("UPDATE titles SET cover_url = 'https://cdn.example/cover.jpg' WHERE id = ?")
        .bind(&t)
        .execute(&state.db)
        .await
        .unwrap();

    let app = arrgh_server::api::router(state);
    let (status, headers, _) = send(&app, &format!("/api/media/cover/{t}")).await;
    assert!(status.is_redirection());
    assert_eq!(
        headers.get("location").unwrap(),
        "https://cdn.example/cover.jpg"
    );
}

// spec: 006/FR-007
#[tokio::test]
async fn serve_cover_returns_local_file_bytes_with_extension_content_type() {
    let state = common::build_state().await;
    let t = common::seed_title(&state, "Naruto", false).await;
    let path = std::env::temp_dir().join(format!("arrgh-media-cover-{}.png", uuid::Uuid::new_v4()));
    tokio::fs::write(&path, [1, 2, 3]).await.unwrap();
    sqlx::query("UPDATE titles SET cover_url = ? WHERE id = ?")
        .bind(path.to_str().unwrap())
        .bind(&t)
        .execute(&state.db)
        .await
        .unwrap();

    let app = arrgh_server::api::router(state);
    let (status, headers, bytes) = send(&app, &format!("/api/media/cover/{t}")).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(headers.get("content-type").unwrap(), "image/png");
    assert_eq!(bytes, vec![1, 2, 3]);
}

// spec: 006/FR-007
#[tokio::test]
async fn serve_cover_falls_back_to_title_meta_cdn_when_local_file_missing() {
    let state = common::build_state().await;
    let t = common::seed_title(&state, "Cached Cover Title", false).await;
    sqlx::query("UPDATE titles SET cover_url = '/nonexistent/cover.png' WHERE id = ?")
        .bind(&t)
        .execute(&state.db)
        .await
        .unwrap();
    let key = arrgh_server::discover::normalize_title("Cached Cover Title");
    sqlx::query(
        "INSERT INTO title_meta (title_key, cover_cdn_url, fetched_at, source, source_id, chapter_count) \
         VALUES (?, 'https://cdn.example/fallback.jpg', datetime('now'), 'mangaupdates', '1', 0)",
    )
    .bind(&key)
    .execute(&state.db)
    .await
    .unwrap();

    let app = arrgh_server::api::router(state.clone());
    let (status, headers, _) = send(&app, &format!("/api/media/cover/{t}")).await;
    assert!(status.is_redirection());
    assert_eq!(
        headers.get("location").unwrap(),
        "https://cdn.example/fallback.jpg"
    );

    let cover_url: Option<String> = sqlx::query_scalar("SELECT cover_url FROM titles WHERE id = ?")
        .bind(&t)
        .fetch_one(&state.db)
        .await
        .unwrap();
    assert_eq!(
        cover_url.as_deref(),
        Some("https://cdn.example/fallback.jpg")
    );
}

// spec: 006/FR-007
#[tokio::test]
async fn serve_cover_not_found_when_nothing_usable() {
    let state = common::build_state().await;
    let t = common::seed_title(&state, "No Cover Title", false).await;

    let app = arrgh_server::api::router(state);
    let (status, _, _) = send(&app, &format!("/api/media/cover/{t}")).await;
    assert_eq!(status, StatusCode::NOT_FOUND);
}
