//! S7 parity check for the background download worker. Mirrors
//! `server-tests/DownloaderTests.cs` (ADR 0033, S7 #129).
//!
//! Same pattern as the .NET suite: seed a `download_queue` row directly,
//! spawn the real worker loop against a mock plugin-host, poll the DB for
//! a terminal status (rather than driving through HTTP or calling `tick`
//! directly — the loop itself, including its 3s cadence, is what's under
//! test). `run_loop_with_interval` with a short interval keeps this fast.

mod common;

use std::sync::{Arc, Mutex};
use std::time::Duration;

use axum::extract::Path as AxPath;
use axum::http::{HeaderMap, StatusCode};
use axum::response::IntoResponse;
use axum::routing::get;
use axum::{Json, Router};
use tokio::sync::Notify;

const TICK: Duration = Duration::from_millis(100);

async fn wait_for_status(pool: &sqlx::SqlitePool, queue_id: &str, timeout: Duration) -> String {
    let deadline = tokio::time::Instant::now() + timeout;
    loop {
        let status: String = sqlx::query_scalar("SELECT status FROM download_queue WHERE id = ?")
            .bind(queue_id)
            .fetch_one(pool)
            .await
            .unwrap();
        if status == "done" || status == "error" {
            return status;
        }
        if tokio::time::Instant::now() >= deadline {
            return status; // let assertions report the stuck status
        }
        tokio::time::sleep(Duration::from_millis(20)).await;
    }
}

/// Fake plugin-host: `mangadex` serves `page_count` image URLs pointing
/// back at itself + a text endpoint; `bad-source` always 502s (simulates
/// an unmatched/broken source); an image path containing "bad-image"
/// 400s. `block` (if set) gates the very first image response, so tests
/// can observe the queue row mid-flight before releasing it.
async fn start_mock(
    page_count: usize,
    block: Option<Arc<Notify>>,
) -> (String, Arc<Mutex<Vec<String>>>) {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    let base = format!("http://{addr}");
    let user_agents: Arc<Mutex<Vec<String>>> = Arc::new(Mutex::new(Vec::new()));

    let base_for_badimg = base.clone();
    let base_for_pages = base.clone();
    let ua_for_pages = user_agents.clone();
    let ua_for_text = user_agents.clone();
    let ua_for_img = user_agents.clone();

    let app = Router::new()
        .route(
            "/mangadex/chapter/{id}/pages",
            get(move |headers: HeaderMap, AxPath(_id): AxPath<String>| {
                let base = base_for_pages.clone();
                let ua = ua_for_pages.clone();
                async move {
                    record_ua(&headers, &ua);
                    let urls: Vec<String> = (0..page_count)
                        .map(|i| format!("{base}/img/{i}.jpg"))
                        .collect();
                    Json(urls).into_response()
                }
            }),
        )
        .route(
            "/badimg/chapter/{id}/pages",
            get(move |AxPath(_id): AxPath<String>| {
                let base = base_for_badimg.clone();
                async move { Json(vec![format!("{base}/img/bad-image.jpg")]).into_response() }
            }),
        )
        .route(
            "/bad-source/chapter/{id}/pages",
            get(|AxPath(_id): AxPath<String>| async {
                (StatusCode::BAD_GATEWAY, "source down").into_response()
            }),
        )
        .route(
            "/mangadex/chapter/{id}/text",
            get(move |headers: HeaderMap, AxPath(_id): AxPath<String>| {
                let ua = ua_for_text.clone();
                async move {
                    record_ua(&headers, &ua);
                    "chapter text content"
                }
            }),
        )
        .route(
            "/img/{name}",
            get(move |headers: HeaderMap, AxPath(name): AxPath<String>| {
                let ua = ua_for_img.clone();
                let block = block.clone();
                async move {
                    record_ua(&headers, &ua);
                    if name.contains("bad-image") {
                        return (StatusCode::BAD_REQUEST, "bad image").into_response();
                    }
                    if name.starts_with("0") {
                        if let Some(n) = block {
                            n.notified().await;
                        }
                    }
                    (
                        StatusCode::OK,
                        [("content-type", "image/jpeg")],
                        vec![0xFFu8, 0xD8, 0xFF, 0xD9],
                    )
                        .into_response()
                }
            }),
        );

    tokio::spawn(async move {
        axum::serve(listener, app).await.unwrap();
    });
    (base, user_agents)
}

fn record_ua(headers: &HeaderMap, ua: &Arc<Mutex<Vec<String>>>) {
    if let Some(h) = headers.get("user-agent") {
        ua.lock().unwrap().push(h.to_str().unwrap().to_string());
    }
}

// ── manga (cbz) path ─────────────────────────────────────────────────────

// spec: 005/FR-004
#[tokio::test]
async fn pages_download_marks_chapter_downloaded_with_cbz() {
    let (mock_url, _) = start_mock(3, None).await;
    let tmp = std::env::temp_dir().join(format!("arrgh-dl-{}", uuid::Uuid::new_v4()));
    let state = common::build_downloader_state(&mock_url, tmp.to_str().unwrap()).await;
    let t = common::seed_title(&state, "Naruto", false).await;
    let c = common::seed_chapter(&state, &t, 1.0, false).await;
    common::add_chapter_source(&state, &c, "mangadex").await;
    let qid = common::seed_queue_item(&state, &c, "Naruto", 1.0, "pending", None).await;

    tokio::spawn(arrgh_server::downloader::run_loop_with_interval(
        state.db.clone(),
        state.http.clone(),
        state.config.plugin_host_url.clone(),
        state.config.download_dir.clone(),
        TICK,
    ));

    let status = wait_for_status(&state.db, &qid, Duration::from_secs(5)).await;
    assert_eq!(status, "done");

    let (downloaded, local_path, page_count): (bool, Option<String>, i64) =
        sqlx::query_as("SELECT downloaded, local_path, page_count FROM chapters WHERE id = ?")
            .bind(&c)
            .fetch_one(&state.db)
            .await
            .unwrap();
    assert!(downloaded);
    assert!(local_path.unwrap().ends_with(".cbz"));
    assert_eq!(page_count, 3);

    let _ = std::fs::remove_dir_all(&tmp);
}

// spec: 002/FR-009, 005/FR-004
#[tokio::test]
async fn no_chapter_sources_sets_error() {
    let (mock_url, _) = start_mock(3, None).await;
    let tmp = std::env::temp_dir().join(format!("arrgh-dl-{}", uuid::Uuid::new_v4()));
    let state = common::build_downloader_state(&mock_url, tmp.to_str().unwrap()).await;
    let t = common::seed_title(&state, "Naruto", false).await;
    let c = common::seed_chapter(&state, &t, 1.0, false).await;
    // no chapter_sources seeded
    let qid = common::seed_queue_item(&state, &c, "Naruto", 1.0, "pending", None).await;

    tokio::spawn(arrgh_server::downloader::run_loop_with_interval(
        state.db.clone(),
        state.http.clone(),
        state.config.plugin_host_url.clone(),
        state.config.download_dir.clone(),
        TICK,
    ));

    let status = wait_for_status(&state.db, &qid, Duration::from_secs(5)).await;
    assert_eq!(status, "error");
    let error: Option<String> = sqlx::query_scalar("SELECT error FROM download_queue WHERE id = ?")
        .bind(&qid)
        .fetch_one(&state.db)
        .await
        .unwrap();
    assert_eq!(error.as_deref(), Some("no chapter sources"));
}

// spec: 002/FR-009, 005/FR-004
#[tokio::test]
async fn all_sources_fail_sets_error() {
    let (mock_url, _) = start_mock(3, None).await;
    let tmp = std::env::temp_dir().join(format!("arrgh-dl-{}", uuid::Uuid::new_v4()));
    let state = common::build_downloader_state(&mock_url, tmp.to_str().unwrap()).await;
    let t = common::seed_title(&state, "Naruto", false).await;
    let c = common::seed_chapter(&state, &t, 1.0, false).await;
    common::add_chapter_source(&state, &c, "unrouted-source").await; // matches no mock route -> 404
    let qid = common::seed_queue_item(&state, &c, "Naruto", 1.0, "pending", None).await;

    tokio::spawn(arrgh_server::downloader::run_loop_with_interval(
        state.db.clone(),
        state.http.clone(),
        state.config.plugin_host_url.clone(),
        state.config.download_dir.clone(),
        TICK,
    ));

    let status = wait_for_status(&state.db, &qid, Duration::from_secs(5)).await;
    assert_eq!(status, "error");
}

#[tokio::test]
async fn error_message_contains_failing_url() {
    let (mock_url, _) = start_mock(3, None).await;
    let tmp = std::env::temp_dir().join(format!("arrgh-dl-{}", uuid::Uuid::new_v4()));
    let state = common::build_downloader_state(&mock_url, tmp.to_str().unwrap()).await;
    let t = common::seed_title(&state, "Naruto", false).await;
    let c = common::seed_chapter(&state, &t, 1.0, false).await;
    common::add_chapter_source(&state, &c, "bad-source").await; // mock 502s this
    let qid = common::seed_queue_item(&state, &c, "Naruto", 1.0, "pending", None).await;

    tokio::spawn(arrgh_server::downloader::run_loop_with_interval(
        state.db.clone(),
        state.http.clone(),
        state.config.plugin_host_url.clone(),
        state.config.download_dir.clone(),
        TICK,
    ));

    wait_for_status(&state.db, &qid, Duration::from_secs(5)).await;
    let error: Option<String> = sqlx::query_scalar("SELECT error FROM download_queue WHERE id = ?")
        .bind(&qid)
        .fetch_one(&state.db)
        .await
        .unwrap();
    let error = error.unwrap();
    assert!(error.contains("/chapter/"), "error was: {error}");
}

#[tokio::test]
async fn image_error_message_contains_failing_image_url() {
    let (mock_url, _) = start_mock(1, None).await;
    let tmp = std::env::temp_dir().join(format!("arrgh-dl-{}", uuid::Uuid::new_v4()));
    let state = common::build_downloader_state(&mock_url, tmp.to_str().unwrap()).await;
    let t = common::seed_title(&state, "Naruto", false).await;
    let c = common::seed_chapter(&state, &t, 1.0, false).await;
    common::add_chapter_source(&state, &c, "badimg").await; // pages endpoint returns a bad-image.jpg URL
    let qid = common::seed_queue_item(&state, &c, "Naruto", 1.0, "pending", None).await;

    tokio::spawn(arrgh_server::downloader::run_loop_with_interval(
        state.db.clone(),
        state.http.clone(),
        state.config.plugin_host_url.clone(),
        state.config.download_dir.clone(),
        TICK,
    ));

    wait_for_status(&state.db, &qid, Duration::from_secs(5)).await;
    let error: Option<String> = sqlx::query_scalar("SELECT error FROM download_queue WHERE id = ?")
        .bind(&qid)
        .fetch_one(&state.db)
        .await
        .unwrap();
    let error = error.unwrap();
    assert!(error.contains("bad-image.jpg"), "error was: {error}");
}

// spec: 002/FR-007, 002/FR-008
#[tokio::test]
async fn priority_fallback_tries_next_source_on_failure() {
    let (mock_url, _) = start_mock(2, None).await;
    let tmp = std::env::temp_dir().join(format!("arrgh-dl-{}", uuid::Uuid::new_v4()));
    let state = common::build_downloader_state(&mock_url, tmp.to_str().unwrap()).await;
    let t = common::seed_title(&state, "Naruto", false).await;
    let c = common::seed_chapter(&state, &t, 1.0, false).await;
    common::add_chapter_source(&state, &c, "bad-source").await;
    common::add_chapter_source(&state, &c, "mangadex").await;
    // bad-source preferred (priority 10 < 20) but 502s; mangadex should be tried next and succeed.
    sqlx::query(
        "INSERT INTO external_sources (id, name, base_url, content_types, enabled, created_at, is_community, priority, source_key, default_explicit) \
         VALUES (?, 'Bad', 'http://x', 'manga', 1, datetime('now'), 0, 10, 'bad-source', 0)",
    )
    .bind(uuid::Uuid::new_v4().to_string())
    .execute(&state.db)
    .await
    .unwrap();
    sqlx::query(
        "INSERT INTO external_sources (id, name, base_url, content_types, enabled, created_at, is_community, priority, source_key, default_explicit) \
         VALUES (?, 'MangaDex', 'http://x', 'manga', 1, datetime('now'), 0, 20, 'mangadex', 0)",
    )
    .bind(uuid::Uuid::new_v4().to_string())
    .execute(&state.db)
    .await
    .unwrap();
    let qid = common::seed_queue_item(&state, &c, "Naruto", 1.0, "pending", None).await;

    tokio::spawn(arrgh_server::downloader::run_loop_with_interval(
        state.db.clone(),
        state.http.clone(),
        state.config.plugin_host_url.clone(),
        state.config.download_dir.clone(),
        TICK,
    ));

    let status = wait_for_status(&state.db, &qid, Duration::from_secs(5)).await;
    assert_eq!(status, "done");
}

// ── novel (text) path ────────────────────────────────────────────────────

#[tokio::test]
async fn text_chapter_downloads_md_file() {
    let (mock_url, _) = start_mock(3, None).await;
    let tmp = std::env::temp_dir().join(format!("arrgh-dl-{}", uuid::Uuid::new_v4()));
    let state = common::build_downloader_state(&mock_url, tmp.to_str().unwrap()).await;
    let t = common::seed_title(&state, "ISSTH", false).await;
    sqlx::query("UPDATE titles SET content_type = 'novel' WHERE id = ?")
        .bind(&t)
        .execute(&state.db)
        .await
        .unwrap();
    let c = common::seed_chapter(&state, &t, 1.0, false).await;
    sqlx::query("UPDATE chapters SET chapter_format = 'text' WHERE id = ?")
        .bind(&c)
        .execute(&state.db)
        .await
        .unwrap();
    common::add_chapter_source(&state, &c, "mangadex").await;
    let qid = common::seed_queue_item(&state, &c, "ISSTH", 1.0, "pending", None).await;

    tokio::spawn(arrgh_server::downloader::run_loop_with_interval(
        state.db.clone(),
        state.http.clone(),
        state.config.plugin_host_url.clone(),
        state.config.download_dir.clone(),
        TICK,
    ));

    let status = wait_for_status(&state.db, &qid, Duration::from_secs(5)).await;
    assert_eq!(status, "done");

    let (downloaded, local_path): (bool, Option<String>) =
        sqlx::query_as("SELECT downloaded, local_path FROM chapters WHERE id = ?")
            .bind(&c)
            .fetch_one(&state.db)
            .await
            .unwrap();
    assert!(downloaded);
    let path = local_path.unwrap();
    assert!(path.ends_with(".md"), "path was: {path}");
    assert_eq!(
        std::fs::read_to_string(&path).unwrap(),
        "chapter text content"
    );

    let _ = std::fs::remove_dir_all(&tmp);
}

// ── worker behaviour ─────────────────────────────────────────────────────

// spec: 005/FR-005
#[tokio::test]
async fn pages_total_and_downloaded_progress_are_tracked() {
    let notify = Arc::new(Notify::new());
    let (mock_url, _) = start_mock(3, Some(notify.clone())).await;
    let tmp = std::env::temp_dir().join(format!("arrgh-dl-{}", uuid::Uuid::new_v4()));
    let state = common::build_downloader_state(&mock_url, tmp.to_str().unwrap()).await;
    let t = common::seed_title(&state, "Naruto", false).await;
    let c = common::seed_chapter(&state, &t, 1.0, false).await;
    common::add_chapter_source(&state, &c, "mangadex").await;
    let qid = common::seed_queue_item(&state, &c, "Naruto", 1.0, "pending", None).await;

    tokio::spawn(arrgh_server::downloader::run_loop_with_interval(
        state.db.clone(),
        state.http.clone(),
        state.config.plugin_host_url.clone(),
        state.config.download_dir.clone(),
        TICK,
    ));

    // pages_total is written before the first page fetch even starts — the mock blocks
    // page 0, so this window proves the total is known up front, not derived from
    // however many pages happen to have completed.
    let deadline = tokio::time::Instant::now() + Duration::from_secs(5);
    loop {
        let pages_total: Option<i64> =
            sqlx::query_scalar("SELECT pages_total FROM download_queue WHERE id = ?")
                .bind(&qid)
                .fetch_one(&state.db)
                .await
                .unwrap();
        if pages_total == Some(3) {
            break;
        }
        assert!(
            tokio::time::Instant::now() < deadline,
            "pages_total never reached 3"
        );
        tokio::time::sleep(Duration::from_millis(10)).await;
    }
    let pages_downloaded: i64 =
        sqlx::query_scalar("SELECT pages_downloaded FROM download_queue WHERE id = ?")
            .bind(&qid)
            .fetch_one(&state.db)
            .await
            .unwrap();
    assert_eq!(pages_downloaded, 0);

    notify.notify_one();
    let status = wait_for_status(&state.db, &qid, Duration::from_secs(5)).await;
    assert_eq!(status, "done");
    let pages_downloaded: i64 =
        sqlx::query_scalar("SELECT pages_downloaded FROM download_queue WHERE id = ?")
            .bind(&qid)
            .fetch_one(&state.db)
            .await
            .unwrap();
    assert_eq!(pages_downloaded, 3);

    let _ = std::fs::remove_dir_all(&tmp);
}

// spec: 005/FR-004, 005/FR-008
#[tokio::test]
async fn status_is_downloading_while_in_progress() {
    let notify = Arc::new(Notify::new());
    let (mock_url, _) = start_mock(1, Some(notify.clone())).await;
    let tmp = std::env::temp_dir().join(format!("arrgh-dl-{}", uuid::Uuid::new_v4()));
    let state = common::build_downloader_state(&mock_url, tmp.to_str().unwrap()).await;
    let t = common::seed_title(&state, "Naruto", false).await;
    let c = common::seed_chapter(&state, &t, 1.0, false).await;
    common::add_chapter_source(&state, &c, "mangadex").await;
    let qid = common::seed_queue_item(&state, &c, "Naruto", 1.0, "pending", None).await;

    tokio::spawn(arrgh_server::downloader::run_loop_with_interval(
        state.db.clone(),
        state.http.clone(),
        state.config.plugin_host_url.clone(),
        state.config.download_dir.clone(),
        TICK,
    ));

    // Poll until the worker claims the item (status flips before the
    // blocked image request even resolves).
    let deadline = tokio::time::Instant::now() + Duration::from_secs(5);
    loop {
        let status: String = sqlx::query_scalar("SELECT status FROM download_queue WHERE id = ?")
            .bind(&qid)
            .fetch_one(&state.db)
            .await
            .unwrap();
        if status == "downloading" {
            break;
        }
        assert!(
            tokio::time::Instant::now() < deadline,
            "never reached downloading"
        );
        tokio::time::sleep(Duration::from_millis(10)).await;
    }

    notify.notify_one();
    let status = wait_for_status(&state.db, &qid, Duration::from_secs(5)).await;
    assert_eq!(status, "done");

    let _ = std::fs::remove_dir_all(&tmp);
}

#[tokio::test]
async fn requests_include_user_agent_header() {
    let (mock_url, user_agents) = start_mock(2, None).await;
    let tmp = std::env::temp_dir().join(format!("arrgh-dl-{}", uuid::Uuid::new_v4()));
    let state = common::build_downloader_state(&mock_url, tmp.to_str().unwrap()).await;
    let t = common::seed_title(&state, "Naruto", false).await;
    let c = common::seed_chapter(&state, &t, 1.0, false).await;
    common::add_chapter_source(&state, &c, "mangadex").await;
    let qid = common::seed_queue_item(&state, &c, "Naruto", 1.0, "pending", None).await;

    tokio::spawn(arrgh_server::downloader::run_loop_with_interval(
        state.db.clone(),
        state.http.clone(),
        state.config.plugin_host_url.clone(),
        state.config.download_dir.clone(),
        TICK,
    ));

    wait_for_status(&state.db, &qid, Duration::from_secs(5)).await;
    let uas = user_agents.lock().unwrap();
    assert!(!uas.is_empty());
    assert!(uas.iter().all(|u| !u.is_empty()));

    let _ = std::fs::remove_dir_all(&tmp);
}

// ── download_workers concurrency (GH #160) ───────────────────────────────

async fn count_downloading(pool: &sqlx::SqlitePool) -> i64 {
    sqlx::query_scalar("SELECT COUNT(*) FROM download_queue WHERE status = 'downloading'")
        .fetch_one(pool)
        .await
        .unwrap()
}

/// Seeds two pending chapters behind a blocked image gate, sets
/// `download_workers`, starts the loop, and returns the peak number of rows
/// seen `downloading` at once over ~15 ticks before releasing the gate.
async fn peak_concurrency(workers: i64) -> i64 {
    let notify = Arc::new(Notify::new());
    let (mock_url, _) = start_mock(1, Some(notify.clone())).await;
    let tmp = std::env::temp_dir().join(format!("arrgh-dl-{}", uuid::Uuid::new_v4()));
    let state = common::build_downloader_state(&mock_url, tmp.to_str().unwrap()).await;
    arrgh_server::settings::set(&state.db, "download_workers", &workers.to_string())
        .await
        .unwrap();
    let t = common::seed_title(&state, "Naruto", false).await;
    let mut qids = Vec::new();
    for n in [1.0, 2.0] {
        let c = common::seed_chapter(&state, &t, n, false).await;
        common::add_chapter_source(&state, &c, "mangadex").await;
        qids.push(common::seed_queue_item(&state, &c, "Naruto", n, "pending", None).await);
    }

    tokio::spawn(arrgh_server::downloader::run_loop_with_interval(
        state.db.clone(),
        state.http.clone(),
        state.config.plugin_host_url.clone(),
        state.config.download_dir.clone(),
        TICK,
    ));

    let mut peak = 0;
    for _ in 0..15 {
        peak = peak.max(count_downloading(&state.db).await);
        tokio::time::sleep(TICK).await;
    }

    // Release every gated image until both items finish.
    let deadline = tokio::time::Instant::now() + Duration::from_secs(5);
    while count_downloading(&state.db).await > 0
        || wait_for_status(&state.db, &qids[1], Duration::ZERO).await == "pending"
    {
        assert!(
            tokio::time::Instant::now() < deadline,
            "downloads never drained"
        );
        notify.notify_waiters();
        tokio::time::sleep(Duration::from_millis(20)).await;
    }
    for q in &qids {
        assert_eq!(
            wait_for_status(&state.db, q, Duration::from_secs(5)).await,
            "done"
        );
    }
    let _ = std::fs::remove_dir_all(&tmp);
    peak
}

// spec: 005/FR-014
#[tokio::test]
async fn download_workers_2_runs_two_items_concurrently() {
    assert_eq!(peak_concurrency(2).await, 2);
}

// spec: 005/FR-014
#[tokio::test]
async fn download_workers_1_runs_items_one_at_a_time() {
    assert_eq!(peak_concurrency(1).await, 1);
}
