use std::collections::HashMap;
use std::sync::Arc;
use std::sync::atomic::AtomicU8;

use axum::Router;
use axum::body::Body;
use axum::http::{Request, StatusCode, header};
use serde_json::{json, Value};
use sqlx::sqlite::{SqliteConnectOptions, SqlitePoolOptions};
use tokio::sync::{Mutex, RwLock};
use tower::ServiceExt;

use arrgh_server::{AppState, Config, api, auth, logging};

// ── helpers ───────────────────────────────────────────────────────────────────

const JWT_SECRET: &str = "integration-test-secret";

async fn build_app() -> Router {
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

    Router::new()
        .nest("/api", api::router(state.clone()))
        .with_state(state)
}

fn admin_token() -> String {
    auth::create_token("user-test", "admin", "admin", false, JWT_SECRET)
        .expect("token")
}

fn member_token() -> String {
    auth::create_token("user-member", "member", "member", false, JWT_SECRET)
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
