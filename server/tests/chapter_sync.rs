//! S5 parity check for chapter-sync (ADR 0033, S5 #127). Mirrors
//! `server-tests/ChapterSyncTests.cs`, using a throwaway HTTP server
//! (`common::start_mock_plugin_host`) in place of `SyncFactory`'s fake
//! `HttpMessageHandler`.

mod common;

use axum::body::Body;
use axum::http::{Request, StatusCode};
use axum::Router;
use http_body_util::BodyExt;
use serde_json::Value;
use tower::ServiceExt; // oneshot

const DEFAULT_CHAPTERS_JSON: &str = r#"[
  {"source_id":"src-ch-1","id":"src-ch-1","number":1.0,"title":"Chapter 1"},
  {"source_id":"src-ch-2","id":"src-ch-2","number":2.0,"title":"Chapter 2"}
]"#;

async fn send(app: &Router, method: &str, uri: &str, token: &str) -> (StatusCode, Value) {
    let req = Request::builder()
        .method(method)
        .uri(uri)
        .header("authorization", format!("Bearer {token}"))
        .body(Body::empty())
        .unwrap();
    let res = app.clone().oneshot(req).await.unwrap();
    let status = res.status();
    let bytes = res.into_body().collect().await.unwrap().to_bytes();
    let json = serde_json::from_slice(&bytes).unwrap_or(Value::Null);
    (status, json)
}

/// Polls `titles.sync_status` for `title_id` until it leaves `"syncing"`.
async fn wait_for_sync_ready(state: &arrgh_server::state::AppState, title_id: &str) {
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(5);
    while std::time::Instant::now() < deadline {
        let status: String = sqlx::query_scalar("SELECT sync_status FROM titles WHERE id = ?")
            .bind(title_id)
            .fetch_one(&state.db)
            .await
            .unwrap();
        if status == "ready" || status == "error" {
            return;
        }
        tokio::time::sleep(std::time::Duration::from_millis(30)).await;
    }
}

async fn setup(chapters_json: &'static str, fail: bool) -> arrgh_server::state::AppState {
    let plugin_host_url = common::start_mock_plugin_host(chapters_json, fail).await;
    common::build_state_with_plugin_host(&plugin_host_url).await
}

// ── chapter creation ─────────────────────────────────────────────────────

#[tokio::test]
async fn sync_creates_chapter_rows() {
    let state = setup(DEFAULT_CHAPTERS_JSON, false).await;
    let admin = common::seed_user(&state, "admin", "admin", true).await;
    let t = common::seed_title(&state, "Naruto", false).await;
    common::seed_user_title(&state, &admin.id, &t).await;
    common::add_title_source(&state, &t, "mangadex").await;
    let token = common::token_for(&admin);
    let app = arrgh_server::api::router(state.clone());

    let (status, _) = send(&app, "POST", &format!("/api/titles/{t}/sync"), &token).await;
    assert_eq!(status, StatusCode::ACCEPTED);
    wait_for_sync_ready(&state, &t).await;

    let numbers: Vec<f64> =
        sqlx::query_scalar("SELECT number FROM chapters WHERE title_id = ? ORDER BY number")
            .bind(&t)
            .fetch_all(&state.db)
            .await
            .unwrap();
    assert_eq!(numbers, vec![1.0, 2.0]);
}

#[tokio::test]
async fn sync_creates_chapter_sources() {
    let state = setup(DEFAULT_CHAPTERS_JSON, false).await;
    let admin = common::seed_user(&state, "admin", "admin", true).await;
    let t = common::seed_title(&state, "Naruto", false).await;
    common::seed_user_title(&state, &admin.id, &t).await;
    common::add_title_source(&state, &t, "mangadex").await;
    let token = common::token_for(&admin);
    let app = arrgh_server::api::router(state.clone());

    send(&app, "POST", &format!("/api/titles/{t}/sync"), &token).await;
    wait_for_sync_ready(&state, &t).await;

    let source_ids: Vec<String> = sqlx::query_scalar(
        "SELECT cs.source_id FROM chapter_sources cs JOIN chapters c ON c.id = cs.chapter_id WHERE c.title_id = ?",
    )
    .bind(&t)
    .fetch_all(&state.db)
    .await
    .unwrap();
    assert_eq!(source_ids.len(), 2);
    assert!(source_ids.contains(&"src-ch-1".to_string()));
    assert!(source_ids.contains(&"src-ch-2".to_string()));
}

#[tokio::test]
async fn sync_sets_status_ready() {
    let state = setup(DEFAULT_CHAPTERS_JSON, false).await;
    let admin = common::seed_user(&state, "admin", "admin", true).await;
    let t = common::seed_title(&state, "Naruto", false).await;
    common::seed_user_title(&state, &admin.id, &t).await;
    common::add_title_source(&state, &t, "mangadex").await;
    let token = common::token_for(&admin);
    let app = arrgh_server::api::router(state.clone());

    send(&app, "POST", &format!("/api/titles/{t}/sync"), &token).await;
    wait_for_sync_ready(&state, &t).await;

    let status: String = sqlx::query_scalar("SELECT sync_status FROM titles WHERE id = ?")
        .bind(&t)
        .fetch_one(&state.db)
        .await
        .unwrap();
    assert_eq!(status, "ready");
}

#[tokio::test]
async fn sync_has_sources_true_in_get_chapters() {
    let state = setup(DEFAULT_CHAPTERS_JSON, false).await;
    let admin = common::seed_user(&state, "admin", "admin", true).await;
    let t = common::seed_title(&state, "Naruto", false).await;
    common::seed_user_title(&state, &admin.id, &t).await;
    common::add_title_source(&state, &t, "mangadex").await;
    let token = common::token_for(&admin);
    let app = arrgh_server::api::router(state.clone());

    send(&app, "POST", &format!("/api/titles/{t}/sync"), &token).await;
    wait_for_sync_ready(&state, &t).await;

    let (_, body) = send(&app, "GET", &format!("/api/chapters/title/{t}"), &token).await;
    let arr = body.as_array().unwrap();
    assert!(!arr.is_empty());
    assert!(arr.iter().all(|c| c["has_sources"] == true));
}

// spec: 002/FR-003
#[tokio::test]
async fn sync_idempotent_no_duplicate_chapters() {
    let state = setup(DEFAULT_CHAPTERS_JSON, false).await;
    let admin = common::seed_user(&state, "admin", "admin", true).await;
    let t = common::seed_title(&state, "Naruto", false).await;
    common::seed_user_title(&state, &admin.id, &t).await;
    common::add_title_source(&state, &t, "mangadex").await;
    let token = common::token_for(&admin);
    let app = arrgh_server::api::router(state.clone());

    send(&app, "POST", &format!("/api/titles/{t}/sync"), &token).await;
    wait_for_sync_ready(&state, &t).await;
    send(&app, "POST", &format!("/api/titles/{t}/sync"), &token).await;
    wait_for_sync_ready(&state, &t).await;

    let chapter_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM chapters WHERE title_id = ?")
        .bind(&t)
        .fetch_one(&state.db)
        .await
        .unwrap();
    assert_eq!(chapter_count, 2);

    let source_count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM chapter_sources cs JOIN chapters c ON c.id = cs.chapter_id WHERE c.title_id = ?",
    )
    .bind(&t)
    .fetch_one(&state.db)
    .await
    .unwrap();
    assert_eq!(source_count, 2);
}

// spec: 002/FR-002, 002/FR-003
#[tokio::test]
async fn sync_two_sources_same_chapter_numbers_one_chapter_row_two_source_links() {
    let state = setup(DEFAULT_CHAPTERS_JSON, false).await;
    let admin = common::seed_user(&state, "admin", "admin", true).await;
    let t = common::seed_title(&state, "Naruto", false).await;
    common::seed_user_title(&state, &admin.id, &t).await;
    common::add_title_source(&state, &t, "mangadex").await;
    common::add_title_source(&state, &t, "mangapill").await;
    let token = common::token_for(&admin);
    let app = arrgh_server::api::router(state.clone());

    send(&app, "POST", &format!("/api/titles/{t}/sync"), &token).await;
    wait_for_sync_ready(&state, &t).await;

    let chapter_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM chapters WHERE title_id = ?")
        .bind(&t)
        .fetch_one(&state.db)
        .await
        .unwrap();
    assert_eq!(chapter_count, 2);

    let source_count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM chapter_sources cs JOIN chapters c ON c.id = cs.chapter_id WHERE c.title_id = ?",
    )
    .bind(&t)
    .fetch_one(&state.db)
    .await
    .unwrap();
    assert_eq!(source_count, 4);
}

// ── chapter_format by content_type ────────────────────────────────────────

async fn assert_chapter_format(content_type: &str, expected_format: &str) {
    let state = setup(DEFAULT_CHAPTERS_JSON, false).await;
    let admin = common::seed_user(&state, "admin", "admin", true).await;
    let t = common::seed_title(&state, "Title", false).await;
    sqlx::query("UPDATE titles SET content_type = ? WHERE id = ?")
        .bind(content_type)
        .bind(&t)
        .execute(&state.db)
        .await
        .unwrap();
    common::seed_user_title(&state, &admin.id, &t).await;
    common::add_title_source(&state, &t, "some-source").await;
    let token = common::token_for(&admin);
    let app = arrgh_server::api::router(state.clone());

    send(&app, "POST", &format!("/api/titles/{t}/sync"), &token).await;
    wait_for_sync_ready(&state, &t).await;

    let formats: Vec<String> =
        sqlx::query_scalar("SELECT chapter_format FROM chapters WHERE title_id = ?")
            .bind(&t)
            .fetch_all(&state.db)
            .await
            .unwrap();
    assert!(!formats.is_empty());
    assert!(formats.iter().all(|f| f == expected_format));
}

#[tokio::test]
async fn sync_novel_chapter_format_is_text() {
    assert_chapter_format("novel", "text").await;
}

#[tokio::test]
async fn sync_manga_chapter_format_is_pages() {
    assert_chapter_format("manga", "pages").await;
}

#[tokio::test]
async fn sync_manhwa_chapter_format_is_pages() {
    assert_chapter_format("manhwa", "pages").await;
}

#[tokio::test]
async fn sync_manhua_chapter_format_is_pages() {
    assert_chapter_format("manhua", "pages").await;
}

// ── plugin-host failure ───────────────────────────────────────────────────

#[tokio::test]
async fn sync_plugin_host_error_sets_status_error() {
    let state = setup(DEFAULT_CHAPTERS_JSON, true).await; // fail = true
    let admin = common::seed_user(&state, "admin", "admin", true).await;
    let t = common::seed_title(&state, "Naruto", false).await;
    common::seed_user_title(&state, &admin.id, &t).await;
    common::add_title_source(&state, &t, "mangadex").await;
    let token = common::token_for(&admin);
    let app = arrgh_server::api::router(state.clone());

    send(&app, "POST", &format!("/api/titles/{t}/sync"), &token).await;
    wait_for_sync_ready(&state, &t).await;

    let status: String = sqlx::query_scalar("SELECT sync_status FROM titles WHERE id = ?")
        .bind(&t)
        .fetch_one(&state.db)
        .await
        .unwrap();
    assert_eq!(status, "error");
}

// ── is_new flag ────────────────────────────────────────────────────────────

// spec: 032/FR-007
#[tokio::test]
async fn sync_chapter_is_new_is_false_after_manual_sync() {
    let state = setup(DEFAULT_CHAPTERS_JSON, false).await;
    let admin = common::seed_user(&state, "admin", "admin", true).await;
    let t = common::seed_title(&state, "Naruto", false).await;
    common::seed_user_title(&state, &admin.id, &t).await;
    common::add_title_source(&state, &t, "mangadex").await;
    let token = common::token_for(&admin);
    let app = arrgh_server::api::router(state.clone());

    send(&app, "POST", &format!("/api/titles/{t}/sync"), &token).await;
    wait_for_sync_ready(&state, &t).await;

    let new_count: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM chapters WHERE title_id = ? AND is_new = 1")
            .bind(&t)
            .fetch_one(&state.db)
            .await
            .unwrap();
    assert_eq!(new_count, 0);
}

// spec: 032/FR-006, 032/FR-007
#[tokio::test]
async fn new_releases_returns_empty_after_sync() {
    let state = setup(DEFAULT_CHAPTERS_JSON, false).await;
    let admin = common::seed_user(&state, "admin", "admin", true).await;
    let t = common::seed_title(&state, "Naruto", false).await;
    common::seed_user_title(&state, &admin.id, &t).await;
    common::add_title_source(&state, &t, "mangadex").await;
    let token = common::token_for(&admin);
    let app = arrgh_server::api::router(state.clone());

    send(&app, "POST", &format!("/api/titles/{t}/sync"), &token).await;
    wait_for_sync_ready(&state, &t).await;

    let (_, body) = send(&app, "GET", "/api/titles/new-releases", &token).await;
    assert_eq!(body.as_array().unwrap().len(), 0);
}

// ── auth / ownership ─────────────────────────────────────────────────────

#[tokio::test]
async fn sync_returns_404_when_caller_does_not_own_title() {
    let state = setup(DEFAULT_CHAPTERS_JSON, false).await;
    let owner = common::seed_user(&state, "admin", "admin", true).await;
    let caller = common::seed_user(&state, "member1", "member", false).await;
    let t = common::seed_title(&state, "Naruto", false).await;
    common::seed_user_title(&state, &owner.id, &t).await;
    common::add_title_source(&state, &t, "mangadex").await;
    let token = common::token_for(&caller);
    let app = arrgh_server::api::router(state);

    let (status, _) = send(&app, "POST", &format!("/api/titles/{t}/sync"), &token).await;
    assert_eq!(status, StatusCode::NOT_FOUND);
}

// ── sync log ──────────────────────────────────────────────────────────────

#[tokio::test]
async fn sync_clears_sync_log_before_new_sync() {
    let state = setup(DEFAULT_CHAPTERS_JSON, false).await;
    let admin = common::seed_user(&state, "admin", "admin", true).await;
    let t = common::seed_title(&state, "Naruto", false).await;
    common::seed_user_title(&state, &admin.id, &t).await;
    common::add_title_source(&state, &t, "mangadex").await;
    common::add_sync_log(&state, &t, "old log entry").await;
    let token = common::token_for(&admin);
    let app = arrgh_server::api::router(state.clone());

    send(&app, "POST", &format!("/api/titles/{t}/sync"), &token).await;
    wait_for_sync_ready(&state, &t).await;

    let messages: Vec<String> =
        sqlx::query_scalar("SELECT message FROM sync_log WHERE title_id = ?")
            .bind(&t)
            .fetch_all(&state.db)
            .await
            .unwrap();
    assert!(!messages.iter().any(|m| m == "old log entry"));
}

// ── duplicate chapter numbers ──────────────────────────────────────────────

#[tokio::test]
async fn sync_duplicate_chapter_numbers_does_not_fail_chapter_sources() {
    const DUP_JSON: &str = r#"[
      {"source_id":"src-ch-63","number":63.0,"title":null},
      {"source_id":"src-ch-63-dup","number":63.0,"title":null}
    ]"#;
    let state = setup(DUP_JSON, false).await;
    let admin = common::seed_user(&state, "admin", "admin", true).await;
    let t = common::seed_title(&state, "Naruto", false).await;
    common::seed_user_title(&state, &admin.id, &t).await;
    common::add_title_source(&state, &t, "mangadex").await;
    let token = common::token_for(&admin);
    let app = arrgh_server::api::router(state.clone());

    send(&app, "POST", &format!("/api/titles/{t}/sync"), &token).await;
    wait_for_sync_ready(&state, &t).await;

    let status: String = sqlx::query_scalar("SELECT sync_status FROM titles WHERE id = ?")
        .bind(&t)
        .fetch_one(&state.db)
        .await
        .unwrap();
    assert_eq!(status, "ready");

    let chapter_ids: Vec<String> = sqlx::query_scalar("SELECT id FROM chapters WHERE title_id = ?")
        .bind(&t)
        .fetch_all(&state.db)
        .await
        .unwrap();
    assert_eq!(chapter_ids.len(), 1);

    let source_ids: Vec<String> =
        sqlx::query_scalar("SELECT source_id FROM chapter_sources WHERE chapter_id = ?")
            .bind(&chapter_ids[0])
            .fetch_all(&state.db)
            .await
            .unwrap();
    assert_eq!(source_ids, vec!["src-ch-63".to_string()]);
}

// ── chapter count logging ──────────────────────────────────────────────────

#[tokio::test]
async fn sync_logs_chapter_count_per_source() {
    let state = setup(DEFAULT_CHAPTERS_JSON, false).await;
    let admin = common::seed_user(&state, "admin", "admin", true).await;
    let t = common::seed_title(&state, "Naruto", false).await;
    common::seed_user_title(&state, &admin.id, &t).await;
    common::add_title_source(&state, &t, "mangadex").await;
    let token = common::token_for(&admin);
    let app = arrgh_server::api::router(state.clone());

    send(&app, "POST", &format!("/api/titles/{t}/sync"), &token).await;
    wait_for_sync_ready(&state, &t).await;

    let messages: Vec<String> =
        sqlx::query_scalar("SELECT message FROM sync_log WHERE title_id = ?")
            .bind(&t)
            .fetch_all(&state.db)
            .await
            .unwrap();
    assert!(messages
        .iter()
        .any(|m| m.contains("Synced") && m.contains("chapter") && m.contains("mangadex")));
}

#[tokio::test]
async fn sync_logs_zero_chapter_count_when_plugin_returns_empty() {
    let state = setup("[]", false).await;
    let admin = common::seed_user(&state, "admin", "admin", true).await;
    let t = common::seed_title(&state, "Naruto", false).await;
    common::seed_user_title(&state, &admin.id, &t).await;
    common::add_title_source(&state, &t, "mangadex").await;
    let token = common::token_for(&admin);
    let app = arrgh_server::api::router(state.clone());

    send(&app, "POST", &format!("/api/titles/{t}/sync"), &token).await;
    wait_for_sync_ready(&state, &t).await;

    let messages: Vec<String> =
        sqlx::query_scalar("SELECT message FROM sync_log WHERE title_id = ?")
            .bind(&t)
            .fetch_all(&state.db)
            .await
            .unwrap();
    assert!(messages
        .iter()
        .any(|m| m.contains('0') && m.contains("chapter") && m.contains("mangadex")));
}

// ── renumbered source chapters (spec 030) ─────────────────────────────────

/// The plugin now numbers "Chapter 2.A" as 2.1 and "Book 5 Recap" as 2.31
/// (it used to report 2 and 5). Same source chapters, new numbers.
const RENUMBERED_JSON: &str = r#"[
  {"source_id":"ch-2a","number":2.1,"title":"Chapter 2.A"},
  {"source_id":"recap","number":2.31,"title":"Book 5 Recap"}
]"#;

async fn numbers(state: &arrgh_server::state::AppState, t: &str) -> Vec<(f64, Option<String>)> {
    sqlx::query_as("SELECT number, title FROM chapters WHERE title_id = ? ORDER BY number")
        .bind(t)
        .fetch_all(&state.db)
        .await
        .unwrap()
}

async fn sync_now(
    state: &arrgh_server::state::AppState,
    admin: &arrgh_server::users::UserRow,
    t: &str,
) {
    let app = arrgh_server::api::router(state.clone());
    let (status, _) = send(
        &app,
        "POST",
        &format!("/api/titles/{t}/sync"),
        &common::token_for(admin),
    )
    .await;
    assert_eq!(status, StatusCode::ACCEPTED);
    wait_for_sync_ready(state, t).await;
}

// spec: 030/FR-002, 019/FR-012
#[tokio::test]
async fn sync_renumbers_a_source_chapter_in_place() {
    let state = setup(RENUMBERED_JSON, false).await;
    let admin = common::seed_user(&state, "admin", "admin", true).await;
    let t = common::seed_title(&state, "DCC", false).await;
    common::seed_user_title(&state, &admin.id, &t).await;
    common::add_title_source(&state, &t, "royalroad").await;
    let old_2 = common::seed_chapter(&state, &t, 2.0, false).await;
    common::add_chapter_source_id(&state, &old_2, "royalroad", "ch-2a").await;
    common::mark_read(&state, &admin.id, &old_2).await;

    sync_now(&state, &admin, &t).await;

    let nums: Vec<f64> = numbers(&state, &t)
        .await
        .into_iter()
        .map(|(n, _)| n)
        .collect();
    assert_eq!(nums, vec![2.1, 2.31]);
    // Same row (reading progress kept), now at 2.1.
    let n: f64 = sqlx::query_scalar("SELECT number FROM chapters WHERE id = ?")
        .bind(&old_2)
        .fetch_one(&state.db)
        .await
        .unwrap();
    assert_eq!(n, 2.1);
}

// spec: 030/FR-002
#[tokio::test]
async fn sync_removes_stale_duplicate_left_by_old_numbering() {
    let state = setup(RENUMBERED_JSON, false).await;
    let admin = common::seed_user(&state, "admin", "admin", true).await;
    let t = common::seed_title(&state, "DCC", false).await;
    common::seed_user_title(&state, &admin.id, &t).await;
    common::add_title_source(&state, &t, "royalroad").await;
    // State after one sync with the new plugin but the old sync code: old rows
    // at 2 and 5 plus new rows at 2.1 and 2.31, each pair on one source chapter.
    for (num, src) in [
        (2.0, "ch-2a"),
        (5.0, "recap"),
        (2.1, "ch-2a"),
        (2.31, "recap"),
    ] {
        let c = common::seed_chapter(&state, &t, num, false).await;
        common::add_chapter_source_id(&state, &c, "royalroad", src).await;
    }

    sync_now(&state, &admin, &t).await;

    let nums: Vec<f64> = numbers(&state, &t)
        .await
        .into_iter()
        .map(|(n, _)| n)
        .collect();
    assert_eq!(nums, vec![2.1, 2.31]);
}

// spec: 030/FR-002
#[tokio::test]
async fn sync_keeps_a_stale_duplicate_that_was_downloaded() {
    let state = setup(RENUMBERED_JSON, false).await;
    let admin = common::seed_user(&state, "admin", "admin", true).await;
    let t = common::seed_title(&state, "DCC", false).await;
    common::seed_user_title(&state, &admin.id, &t).await;
    common::add_title_source(&state, &t, "royalroad").await;
    let old = common::seed_chapter(&state, &t, 2.0, true).await; // downloaded
    common::add_chapter_source_id(&state, &old, "royalroad", "ch-2a").await;
    let new = common::seed_chapter(&state, &t, 2.1, false).await;
    common::add_chapter_source_id(&state, &new, "royalroad", "ch-2a").await;

    sync_now(&state, &admin, &t).await;

    let nums: Vec<f64> = numbers(&state, &t)
        .await
        .into_iter()
        .map(|(n, _)| n)
        .collect();
    assert_eq!(nums, vec![2.0, 2.1, 2.31]);
}

/// GH #173: WuxiaWorld used to store guessed ids ("novel/chapter/2") that never
/// load; it now reports the real slug for the same chapter. Re-sync must replace
/// the stale link, or the downloader keeps hitting the dead URL.
#[tokio::test]
async fn sync_updates_a_changed_source_id_for_the_same_chapter() {
    const JSON: &str =
        r#"[{"source_id":"issth/issth-book-1-chapter-2","number":2.0,"title":"Chapter 2"}]"#;
    let state = setup(JSON, false).await;
    let admin = common::seed_user(&state, "admin", "admin", true).await;
    let t = common::seed_title(&state, "ISSTH", false).await;
    common::seed_user_title(&state, &admin.id, &t).await;
    common::add_title_source(&state, &t, "wuxiaworld").await;
    let ch = common::seed_chapter(&state, &t, 2.0, false).await;
    common::add_chapter_source_id(&state, &ch, "wuxiaworld", "issth/chapter/2").await;

    sync_now(&state, &admin, &t).await;

    let ids: Vec<String> = sqlx::query_scalar(
        "SELECT cs.source_id FROM chapter_sources cs JOIN chapters c ON c.id = cs.chapter_id WHERE c.title_id = ?",
    )
    .bind(&t)
    .fetch_all(&state.db)
    .await
    .unwrap();
    assert_eq!(ids, vec!["issth/issth-book-1-chapter-2".to_string()]);
}

// ── new-release marking (GH #210) ────────────────────────────────────────

async fn new_flags(state: &arrgh_server::state::AppState, title_id: &str) -> Vec<(f64, bool)> {
    sqlx::query_as("SELECT number, is_new FROM chapters WHERE title_id = ? ORDER BY number")
        .bind(title_id)
        .fetch_all(&state.db)
        .await
        .unwrap()
}

/// One sync run with two sources, through the same path as manual/scheduled sync.
async fn sync_once(state: &arrgh_server::state::AppState, title_id: &str) {
    let links = [
        ("mangadex".to_string(), "src-1".to_string()),
        ("mangapill".to_string(), "src-1".to_string()),
    ];
    arrgh_server::titles::run_sync(
        &state.db,
        &state.http,
        &state.config.plugin_host_url,
        title_id,
        "manga",
        &links,
    )
    .await;
}

// spec: 032/FR-007
#[tokio::test]
async fn initial_import_marks_nothing_new() {
    let state = setup(DEFAULT_CHAPTERS_JSON, false).await;
    let t = common::seed_title(&state, "Berserk", false).await;
    sync_once(&state, &t).await;
    assert_eq!(
        new_flags(&state, &t).await,
        vec![(1.0, false), (2.0, false)]
    );
}

// spec: 032/FR-007
#[tokio::test]
async fn resync_marks_only_newly_found_chapters_new() {
    let state = setup(DEFAULT_CHAPTERS_JSON, false).await;
    let t = common::seed_title(&state, "Berserk", false).await;
    common::seed_chapter(&state, &t, 1.0, false).await;
    sync_once(&state, &t).await;
    assert_eq!(new_flags(&state, &t).await, vec![(1.0, false), (2.0, true)]);
}

/// GH #225: a title that fell behind (or gained a source with a full back-catalog)
/// must not surface its whole backlog as "new" — only the actual latest chapter.
// spec: 032/FR-007
#[tokio::test]
async fn resync_with_a_backlog_marks_only_the_newest_chapter() {
    const BACKLOG_JSON: &str = r#"[
      {"source_id":"src-ch-1","id":"src-ch-1","number":1.0,"title":"Chapter 1"},
      {"source_id":"src-ch-2","id":"src-ch-2","number":2.0,"title":"Chapter 2"},
      {"source_id":"src-ch-3","id":"src-ch-3","number":3.0,"title":"Chapter 3"},
      {"source_id":"src-ch-4","id":"src-ch-4","number":4.0,"title":"Chapter 4"}
    ]"#;
    let state = setup(BACKLOG_JSON, false).await;
    let t = common::seed_title(&state, "One Piece", false).await;
    common::seed_chapter(&state, &t, 1.0, false).await;
    sync_once(&state, &t).await;
    assert_eq!(
        new_flags(&state, &t).await,
        vec![(1.0, false), (2.0, false), (3.0, false), (4.0, true)]
    );
}
