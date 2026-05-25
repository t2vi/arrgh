use std::collections::HashMap;
use std::sync::Arc;
use std::sync::atomic::AtomicU8;

use async_trait::async_trait;
use axum::Router;
use axum::body::Body;
use axum::http::{Request, StatusCode, header};
use serde_json::{json, Value};
use sqlx::sqlite::{SqliteConnectOptions, SqlitePoolOptions};
use tokio::sync::{Mutex, RwLock};
use tower::ServiceExt;

use arrgh_server::{AppState, Config, api, auth, logging};
use arrgh_server::indexer::source::{MangaResult, PageUrl, Source};

// ── helpers ───────────────────────────────────────────────────────────────────

const JWT_SECRET: &str = "integration-test-secret";
const ADMIN_UID: &str = "user-test";

async fn build_state() -> (Router, AppState) {
    let opts = SqliteConnectOptions::new()
        .filename(":memory:")
        .create_if_missing(true);
    let pool = SqlitePoolOptions::new()
        .max_connections(1)
        .connect_with(opts)
        .await
        .expect("in-memory sqlite");

    sqlx::migrate!("./migrations")
        .run(&pool)
        .await
        .expect("migrations");

    let config = Arc::new(Config {
        database_url: ":memory:".into(),
        download_dir: "/tmp/test-downloads".into(),
        bind_addr: None,
        plugin_host_url: "http://localhost:4000".into(),
        plugin_index_url: "file:///dev/null".into(),
        index_interval_hours: 6,
    });

    let log_level = Arc::new(AtomicU8::new(logging::LEVEL_INFO));
    let log_buffer = logging::new_buffer();

    let state = AppState {
        db: pool,
        config,
        jwt_secret: Arc::new(JWT_SECRET.to_string()),
        sources: Arc::new(RwLock::new(HashMap::new())),
        registry_lock: Arc::new(Mutex::new(())),
        page_cache: Arc::new(Mutex::new(HashMap::new())),
        trending_cache: Arc::new(Mutex::new(None)),
        log_buffer,
        log_level,
        http: reqwest::Client::new(),
        update_cache: api::version::new_cache(),
    };

    // Seed the admin user so FK constraints on user_manga are satisfied
    let now = chrono::Utc::now().to_rfc3339();
    sqlx::query(
        "INSERT INTO users (id, username, password_hash, created_at) VALUES (?, ?, ?, ?)"
    )
    .bind(ADMIN_UID).bind("admin").bind("test-hash").bind(&now)
    .execute(&state.db).await.expect("seed admin user");

    let router = Router::new()
        .nest("/api", api::router(state.clone()))
        .with_state(state.clone());

    (router, state)
}

async fn build_app() -> Router {
    build_state().await.0
}

// ── DB seed helpers ───────────────────────────────────────────────────────────

async fn seed_manga(pool: &sqlx::SqlitePool, id: &str) {
    let now = chrono::Utc::now().to_rfc3339();
    sqlx::query(
        "INSERT INTO manga (id, title, status, sync_status, content_type, is_explicit, created_at, updated_at) \
         VALUES (?, 'Test', 'unknown', 'ready', 'manga', 0, ?, ?)"
    )
    .bind(id).bind(&now).bind(&now)
    .execute(pool).await.unwrap();
}

async fn seed_user_manga(pool: &sqlx::SqlitePool, manga_id: &str) {
    let now = chrono::Utc::now().to_rfc3339();
    sqlx::query("INSERT OR IGNORE INTO user_manga (user_id, manga_id, added_at) VALUES (?, ?, ?)")
        .bind(ADMIN_UID).bind(manga_id).bind(&now)
        .execute(pool).await.unwrap();
}

async fn seed_manga_source(pool: &sqlx::SqlitePool, manga_id: &str, source: &str, source_id: &str) {
    let id = uuid::Uuid::new_v4().to_string();
    let now = chrono::Utc::now().to_rfc3339();
    sqlx::query(
        "INSERT INTO manga_sources (id, manga_id, source, source_id, discovered_at) VALUES (?, ?, ?, ?, ?)"
    )
    .bind(&id).bind(manga_id).bind(source).bind(source_id).bind(&now)
    .execute(pool).await.unwrap();
}

async fn seed_chapter(pool: &sqlx::SqlitePool, chapter_id: &str, manga_id: &str, number: f64) {
    let now = chrono::Utc::now().to_rfc3339();
    sqlx::query(
        "INSERT INTO chapters (id, manga_id, number, page_count, downloaded, chapter_format, created_at) \
         VALUES (?, ?, ?, 0, 0, 'pages', ?)"
    )
    .bind(chapter_id).bind(manga_id).bind(number).bind(&now)
    .execute(pool).await.unwrap();
}

async fn seed_chapter_source(pool: &sqlx::SqlitePool, chapter_id: &str, source: &str, source_id: &str) {
    let id = uuid::Uuid::new_v4().to_string();
    sqlx::query("INSERT INTO chapter_sources (id, chapter_id, source, source_id) VALUES (?, ?, ?, ?)")
        .bind(&id).bind(chapter_id).bind(source).bind(source_id)
        .execute(pool).await.unwrap();
}

async fn seed_explicit_manga(pool: &sqlx::SqlitePool, id: &str) {
    let now = chrono::Utc::now().to_rfc3339();
    sqlx::query(
        "INSERT INTO manga (id, title, status, sync_status, content_type, is_explicit, created_at, updated_at) \
         VALUES (?, 'Explicit Manga', 'unknown', 'ready', 'manga', 1, ?, ?)"
    )
    .bind(id).bind(&now).bind(&now)
    .execute(pool).await.unwrap();
}

async fn seed_member_user(pool: &sqlx::SqlitePool) {
    let now = chrono::Utc::now().to_rfc3339();
    sqlx::query(
        "INSERT OR IGNORE INTO users (id, username, password_hash, role, created_at) VALUES (?, ?, ?, ?, ?)"
    )
    .bind("user-member").bind("member").bind("test-hash").bind("member").bind(&now)
    .execute(pool).await.unwrap();
}

async fn seed_queue_item(pool: &sqlx::SqlitePool, queue_id: &str, chapter_id: &str, queued_by: Option<&str>) {
    let now = chrono::Utc::now().to_rfc3339();
    sqlx::query(
        "INSERT INTO download_queue (id, chapter_id, manga_title, chapter_num, status, created_at, updated_at, queued_by) \
         VALUES (?, ?, 'Test', 1.0, 'pending', ?, ?, ?)"
    )
    .bind(queue_id).bind(chapter_id).bind(&now).bind(&now).bind(queued_by)
    .execute(pool).await.unwrap();
}

// ── MockSource ────────────────────────────────────────────────────────────────

struct MockSource { id: String }

#[async_trait]
impl Source for MockSource {
    fn id(&self) -> &str { &self.id }
    async fn search(&self, _: &str) -> anyhow::Result<Vec<MangaResult>> { Ok(vec![]) }
    async fn sync_chapters(&self, _: &sqlx::SqlitePool, _: &str, _: &str) -> anyhow::Result<usize> { Ok(0) }
    async fn get_page_urls(&self, _: &str) -> anyhow::Result<Vec<PageUrl>> { Ok(vec![]) }
    async fn fetch_cover(&self, _: &str) -> anyhow::Result<Vec<u8>> { Ok(vec![]) }
}

fn admin_token() -> String {
    auth::create_token("user-test", "admin", "admin", false, JWT_SECRET)
        .expect("token")
}

fn member_token() -> String {
    auth::create_token("user-member", "member", "member", false, JWT_SECRET)
        .expect("token")
}

fn member_explicit_token() -> String {
    auth::create_token("user-member", "member", "member", true, JWT_SECRET)
        .expect("token")
}

async fn req_get(app: &Router, path: &str, token: Option<&str>) -> (StatusCode, Value) {
    let mut req = Request::builder().method("GET").uri(path);
    if let Some(t) = token {
        req = req.header(header::AUTHORIZATION, format!("Bearer {}", t));
    }
    let resp = app
        .clone()
        .oneshot(req.body(Body::empty()).unwrap())
        .await
        .unwrap();
    let status = resp.status();
    let bytes = axum::body::to_bytes(resp.into_body(), usize::MAX).await.unwrap();
    let body = serde_json::from_slice(&bytes).unwrap_or(Value::Null);
    (status, body)
}

async fn req_post(app: &Router, path: &str, token: Option<&str>, payload: Value) -> (StatusCode, Value) {
    let mut req = Request::builder()
        .method("POST")
        .uri(path)
        .header(header::CONTENT_TYPE, "application/json");
    if let Some(t) = token {
        req = req.header(header::AUTHORIZATION, format!("Bearer {}", t));
    }
    let body = Body::from(payload.to_string());
    let resp = app
        .clone()
        .oneshot(req.body(body).unwrap())
        .await
        .unwrap();
    let status = resp.status();
    let bytes = axum::body::to_bytes(resp.into_body(), usize::MAX).await.unwrap();
    let body = serde_json::from_slice(&bytes).unwrap_or(Value::Null);
    (status, body)
}

async fn req_patch(app: &Router, path: &str, token: Option<&str>, payload: Value) -> (StatusCode, Value) {
    let mut req = Request::builder()
        .method("PATCH")
        .uri(path)
        .header(header::CONTENT_TYPE, "application/json");
    if let Some(t) = token {
        req = req.header(header::AUTHORIZATION, format!("Bearer {}", t));
    }
    let body = Body::from(payload.to_string());
    let resp = app
        .clone()
        .oneshot(req.body(body).unwrap())
        .await
        .unwrap();
    let status = resp.status();
    let bytes = axum::body::to_bytes(resp.into_body(), usize::MAX).await.unwrap();
    let body = serde_json::from_slice(&bytes).unwrap_or(Value::Null);
    (status, body)
}

async fn req_delete(app: &Router, path: &str, token: Option<&str>) -> StatusCode {
    let mut req = Request::builder().method("DELETE").uri(path);
    if let Some(t) = token {
        req = req.header(header::AUTHORIZATION, format!("Bearer {}", t));
    }
    app.clone()
        .oneshot(req.body(Body::empty()).unwrap())
        .await
        .unwrap()
        .status()
}

// ── auth ─────────────────────────────────────────────────────────────────────

#[tokio::test]
async fn no_token_returns_401() {
    let app = build_app().await;
    let (status, _) = req_get(&app, "/api/settings", None).await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn valid_token_lets_request_through() {
    let app = build_app().await;
    let token = admin_token();
    let (status, _) = req_get(&app, "/api/settings", Some(&token)).await;
    assert_eq!(status, StatusCode::OK);
}

#[tokio::test]
async fn media_route_bypasses_auth() {
    let app = build_app().await;
    // Media routes require no auth — returns 404 (not found) not 401
    let (status, _) = req_get(&app, "/api/media/cover/nonexistent", None).await;
    assert_ne!(status, StatusCode::UNAUTHORIZED);
}

// ── settings ─────────────────────────────────────────────────────────────────

#[tokio::test]
async fn get_settings_returns_ok() {
    let app = build_app().await;
    let token = admin_token();
    let (status, body) = req_get(&app, "/api/settings", Some(&token)).await;
    assert_eq!(status, StatusCode::OK);
    assert!(body.is_object());
}

#[tokio::test]
async fn post_settings_persists_and_returns_updated() {
    let app = build_app().await;
    let token = admin_token();

    let (status, body) = req_post(
        &app,
        "/api/settings",
        Some(&token),
        json!({ "download_workers": 4, "reader_mode": "scroll" }),
    ).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["download_workers"], 4);
    assert_eq!(body["reader_mode"], "scroll");
}

#[tokio::test]
async fn post_settings_invalid_reader_mode_returns_422() {
    let app = build_app().await;
    let token = admin_token();

    let (status, _) = req_post(
        &app,
        "/api/settings",
        Some(&token),
        json!({ "reader_mode": "invalid_mode" }),
    ).await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
}

// ── logs ─────────────────────────────────────────────────────────────────────

#[tokio::test]
async fn get_logs_returns_empty_array_on_fresh_buffer() {
    let app = build_app().await;
    let token = admin_token();
    let (status, body) = req_get(&app, "/api/logs", Some(&token)).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body, json!([]));
}

#[tokio::test]
async fn set_log_level_requires_admin() {
    let app = build_app().await;
    let token = member_token();
    let (status, _) = req_patch(
        &app,
        "/api/logs/level",
        Some(&token),
        json!({ "level": "DEBUG" }),
    ).await;
    assert_eq!(status, StatusCode::FORBIDDEN);
}

#[tokio::test]
async fn admin_can_set_log_level() {
    let app = build_app().await;
    let token = admin_token();
    let (status, _) = req_patch(
        &app,
        "/api/logs/level",
        Some(&token),
        json!({ "level": "DEBUG" }),
    ).await;
    assert_eq!(status, StatusCode::NO_CONTENT);
}

// ── version ───────────────────────────────────────────────────────────────────

#[tokio::test]
async fn version_returns_current_without_latest_when_check_disabled() {
    let app = build_app().await;
    let token = admin_token();
    let (status, body) = req_get(&app, "/api/version", Some(&token)).await;
    assert_eq!(status, StatusCode::OK);
    assert!(body.get("current").is_some());
    // Update check is disabled by default → latest is null
    assert_eq!(body["latest"], Value::Null);
}

// ── manga schema: is_local ────────────────────────────────────────────────────

#[tokio::test]
async fn manga_is_local_when_no_source_links() {
    let (app, state) = build_state().await;
    let token = admin_token();

    seed_manga(&state.db, "m-local").await;
    seed_user_manga(&state.db, "m-local").await;

    let (status, body) = req_get(&app, "/api/manga/m-local", Some(&token)).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["is_local"], true, "manga with no manga_sources rows is local");
}

#[tokio::test]
async fn manga_is_not_local_when_source_links_exist() {
    let (app, state) = build_state().await;
    let token = admin_token();

    seed_manga(&state.db, "m-remote").await;
    seed_user_manga(&state.db, "m-remote").await;
    seed_manga_source(&state.db, "m-remote", "mangadex", "src-001").await;

    let (status, body) = req_get(&app, "/api/manga/m-remote", Some(&token)).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["is_local"], false, "manga with manga_sources rows is not local");
}

// ── chapters schema: has_sources ─────────────────────────────────────────────

#[tokio::test]
async fn chapter_has_sources_false_without_chapter_sources() {
    let (app, state) = build_state().await;
    let token = admin_token();

    seed_manga(&state.db, "m-ch1").await;
    seed_chapter(&state.db, "ch-no-src", "m-ch1", 1.0).await;

    let (status, body) = req_get(&app, "/api/chapters/manga/m-ch1", Some(&token)).await;
    assert_eq!(status, StatusCode::OK);
    let chapters = body.as_array().unwrap();
    assert_eq!(chapters.len(), 1);
    assert_eq!(chapters[0]["has_sources"], false, "chapter with no chapter_sources has has_sources=false");
}

#[tokio::test]
async fn chapter_has_sources_true_with_chapter_sources() {
    let (app, state) = build_state().await;
    let token = admin_token();

    seed_manga(&state.db, "m-ch2").await;
    seed_chapter(&state.db, "ch-with-src", "m-ch2", 1.0).await;
    seed_chapter_source(&state.db, "ch-with-src", "mangadex", "cs-001").await;

    let (status, body) = req_get(&app, "/api/chapters/manga/m-ch2", Some(&token)).await;
    assert_eq!(status, StatusCode::OK);
    let chapters = body.as_array().unwrap();
    assert_eq!(chapters.len(), 1);
    assert_eq!(chapters[0]["has_sources"], true, "chapter with chapter_sources has has_sources=true");
}

// ── queue_download uses chapter_sources guard ─────────────────────────────────

#[tokio::test]
async fn queue_download_returns_404_without_chapter_sources() {
    let (app, state) = build_state().await;
    let token = admin_token();

    seed_manga(&state.db, "m-dl1").await;
    seed_chapter(&state.db, "ch-dl-none", "m-dl1", 1.0).await;

    let (status, _) = req_post(&app, "/api/chapters/ch-dl-none/download", Some(&token), json!({})).await;
    assert_eq!(status, StatusCode::NOT_FOUND, "no chapter_sources → cannot queue download");
}

#[tokio::test]
async fn queue_download_returns_202_with_chapter_sources() {
    let (app, state) = build_state().await;
    let token = admin_token();

    seed_manga(&state.db, "m-dl2").await;
    seed_chapter(&state.db, "ch-dl-ok", "m-dl2", 1.0).await;
    seed_chapter_source(&state.db, "ch-dl-ok", "mangadex", "dl-src-001").await;

    let (status, _) = req_post(&app, "/api/chapters/ch-dl-ok/download", Some(&token), json!({})).await;
    assert_eq!(status, StatusCode::ACCEPTED, "chapter_sources present → download queued");
}

// ── sync_manga uses manga_sources ─────────────────────────────────────────────

#[tokio::test]
async fn sync_manga_returns_404_when_no_source_links() {
    let (app, state) = build_state().await;
    let token = admin_token();

    seed_manga(&state.db, "m-sync1").await;
    seed_user_manga(&state.db, "m-sync1").await;

    let (status, _) = req_post(&app, "/api/manga/m-sync1/sync", Some(&token), json!({})).await;
    assert_eq!(status, StatusCode::NOT_FOUND, "local-only manga has no source links to sync");
}

#[tokio::test]
async fn sync_manga_returns_202_when_source_links_exist() {
    let (app, state) = build_state().await;
    let token = admin_token();

    seed_manga(&state.db, "m-sync2").await;
    seed_user_manga(&state.db, "m-sync2").await;
    seed_manga_source(&state.db, "m-sync2", "mangadex", "src-sync").await;

    let (status, _) = req_post(&app, "/api/manga/m-sync2/sync", Some(&token), json!({})).await;
    assert_eq!(status, StatusCode::ACCEPTED);
}

// ── add_manga multi-source ────────────────────────────────────────────────────

#[tokio::test]
async fn add_manga_returns_400_when_source_not_in_registry() {
    let (app, _state) = build_state().await;
    let token = admin_token();

    let (status, _) = req_post(&app, "/api/discover/add", Some(&token), json!({
        "source": "nonexistent-source",
        "source_id": "m-001",
        "title": "Test Manga",
        "status": "ongoing",
        "content_type": "manga",
    })).await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn add_manga_creates_manga_source_link_for_primary() {
    let (app, state) = build_state().await;
    let token = admin_token();

    state.sources.write().await.insert("mock".into(), Arc::new(MockSource { id: "mock".into() }));

    let (status, body) = req_post(&app, "/api/discover/add", Some(&token), json!({
        "source": "mock",
        "source_id": "m-primary",
        "title": "Test Manga",
        "status": "ongoing",
        "content_type": "manga",
    })).await;
    assert_eq!(status, StatusCode::OK);

    let manga_id = body["id"].as_str().unwrap();
    let exists: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM manga_sources WHERE manga_id = ? AND source = 'mock' AND source_id = 'm-primary'"
    )
    .bind(manga_id).fetch_one(&state.db).await.unwrap();
    assert_eq!(exists, 1, "primary source link created in manga_sources");
}

#[tokio::test]
async fn add_manga_creates_source_links_for_alternatives() {
    let (app, state) = build_state().await;
    let token = admin_token();

    state.sources.write().await.insert("mock".into(), Arc::new(MockSource { id: "mock".into() }));

    let (status, body) = req_post(&app, "/api/discover/add", Some(&token), json!({
        "source": "mock",
        "source_id": "m-primary",
        "title": "Test Manga",
        "status": "ongoing",
        "content_type": "manga",
        "alternatives": [
            { "source": "mock-alt", "source_name": "Mock Alt", "id": "m-alt-1", "cover_url": null, "status": "ongoing" }
        ]
    })).await;
    assert_eq!(status, StatusCode::OK);

    let manga_id = body["id"].as_str().unwrap();
    let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM manga_sources WHERE manga_id = ?")
        .bind(manga_id).fetch_one(&state.db).await.unwrap();
    assert_eq!(count, 2, "primary + one alternative stored in manga_sources");
}

#[tokio::test]
async fn add_manga_deduplicates_on_same_source_link() {
    let (app, state) = build_state().await;
    let token = admin_token();

    state.sources.write().await.insert("mock".into(), Arc::new(MockSource { id: "mock".into() }));

    let payload = json!({
        "source": "mock",
        "source_id": "m-dedup",
        "title": "Dedup Manga",
        "status": "ongoing",
        "content_type": "manga",
    });

    let (_, body1) = req_post(&app, "/api/discover/add", Some(&token), payload.clone()).await;
    let (_, body2) = req_post(&app, "/api/discover/add", Some(&token), payload).await;

    assert_eq!(body1["id"], body2["id"], "same source link → same manga returned");

    let count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM manga_sources WHERE source = 'mock' AND source_id = 'm-dedup'"
    )
    .fetch_one(&state.db).await.unwrap();
    assert_eq!(count, 1, "only one manga_sources row despite two add calls");
}

// ── queue: explicit filter ────────────────────────────────────────────────────

#[tokio::test]
async fn queue_hides_explicit_items_from_member_without_permission() {
    let (app, state) = build_state().await;

    seed_explicit_manga(&state.db, "m-expl-q1").await;
    seed_chapter(&state.db, "ch-expl-q1", "m-expl-q1", 1.0).await;
    seed_queue_item(&state.db, "qi-expl-1", "ch-expl-q1", None).await;

    let token = member_token(); // allow_explicit = false
    let (status, body) = req_get(&app, "/api/queue", Some(&token)).await;
    assert_eq!(status, StatusCode::OK);
    let items = body.as_array().unwrap();
    assert!(items.is_empty(), "member without explicit perm must not see explicit queue items");
}

#[tokio::test]
async fn queue_shows_explicit_items_to_member_with_permission() {
    let (app, state) = build_state().await;

    seed_explicit_manga(&state.db, "m-expl-q2").await;
    seed_chapter(&state.db, "ch-expl-q2", "m-expl-q2", 1.0).await;
    seed_queue_item(&state.db, "qi-expl-2", "ch-expl-q2", None).await;

    let token = member_explicit_token(); // allow_explicit = true
    let (status, body) = req_get(&app, "/api/queue", Some(&token)).await;
    assert_eq!(status, StatusCode::OK);
    let items = body.as_array().unwrap();
    assert_eq!(items.len(), 1, "member with explicit perm sees explicit queue items");
}

#[tokio::test]
async fn queue_shows_explicit_items_to_admin() {
    let (app, state) = build_state().await;

    seed_explicit_manga(&state.db, "m-expl-q3").await;
    seed_chapter(&state.db, "ch-expl-q3", "m-expl-q3", 1.0).await;
    seed_queue_item(&state.db, "qi-expl-3", "ch-expl-q3", None).await;

    let token = admin_token();
    let (status, body) = req_get(&app, "/api/queue", Some(&token)).await;
    assert_eq!(status, StatusCode::OK);
    let items = body.as_array().unwrap();
    assert_eq!(items.len(), 1, "admin sees explicit queue items");
}

// ── queue: clear_completed admin-only ────────────────────────────────────────

#[tokio::test]
async fn clear_completed_forbidden_for_member() {
    let (app, _state) = build_state().await;
    let token = member_token();
    let status = req_delete(&app, "/api/queue/completed", Some(&token)).await;
    assert_eq!(status, StatusCode::FORBIDDEN, "members cannot clear completed queue items");
}

#[tokio::test]
async fn clear_completed_allowed_for_admin() {
    let (app, _state) = build_state().await;
    let token = admin_token();
    let status = req_delete(&app, "/api/queue/completed", Some(&token)).await;
    assert_eq!(status, StatusCode::NO_CONTENT, "admin can clear completed queue items");
}

// ── queue: cancel ownership ───────────────────────────────────────────────────

#[tokio::test]
async fn member_cannot_cancel_another_users_queue_item() {
    let (app, state) = build_state().await;
    seed_member_user(&state.db).await;

    seed_manga(&state.db, "m-cancel-1").await;
    seed_chapter(&state.db, "ch-cancel-1", "m-cancel-1", 1.0).await;
    // queued by admin (ADMIN_UID), not the member
    seed_queue_item(&state.db, "qi-cancel-1", "ch-cancel-1", Some(ADMIN_UID)).await;

    let token = member_token();
    let status = req_delete(&app, "/api/queue/qi-cancel-1", Some(&token)).await;
    assert_eq!(status, StatusCode::FORBIDDEN, "member cannot cancel admin's queue item");
}

#[tokio::test]
async fn member_can_cancel_own_queue_item() {
    let (app, state) = build_state().await;
    seed_member_user(&state.db).await;

    seed_manga(&state.db, "m-cancel-2").await;
    seed_chapter(&state.db, "ch-cancel-2", "m-cancel-2", 1.0).await;
    seed_queue_item(&state.db, "qi-cancel-2", "ch-cancel-2", Some("user-member")).await;

    let token = member_token();
    let status = req_delete(&app, "/api/queue/qi-cancel-2", Some(&token)).await;
    assert_eq!(status, StatusCode::NO_CONTENT, "member can cancel their own queue item");
}

#[tokio::test]
async fn admin_can_cancel_any_queue_item() {
    let (app, state) = build_state().await;
    seed_member_user(&state.db).await;

    seed_manga(&state.db, "m-cancel-3").await;
    seed_chapter(&state.db, "ch-cancel-3", "m-cancel-3", 1.0).await;
    seed_queue_item(&state.db, "qi-cancel-3", "ch-cancel-3", Some("user-member")).await;

    let token = admin_token();
    let status = req_delete(&app, "/api/queue/qi-cancel-3", Some(&token)).await;
    assert_eq!(status, StatusCode::NO_CONTENT, "admin can cancel any queue item");
}

// ── remove_manga: delete_files admin-only ─────────────────────────────────────

#[tokio::test]
async fn member_cannot_delete_files_on_remove() {
    let (app, state) = build_state().await;
    seed_member_user(&state.db).await;

    // Seed manga subscribed to by the member user
    seed_manga(&state.db, "m-rmfiles-1").await;
    let now = chrono::Utc::now().to_rfc3339();
    sqlx::query("INSERT INTO user_manga (user_id, manga_id, added_at) VALUES (?, ?, ?)")
        .bind("user-member").bind("m-rmfiles-1").bind(&now)
        .execute(&state.db).await.unwrap();

    let token = member_token();
    // delete_files=true should be silently ignored — member still gets 204
    let status = req_delete(&app, "/api/manga/m-rmfiles-1?delete_files=true", Some(&token)).await;
    assert_eq!(status, StatusCode::NO_CONTENT, "member remove returns 204 (delete_files silently ignored)");
}
