//! S9 parity check for `/api/plugins`. Mirrors `server-tests/PluginsTests.cs`
//! + `api-tests/tests/plugins.hurl` (ADR 0033, S9 #131).

mod common;

use axum::body::Body;
use axum::http::{Request, StatusCode};
use axum::Router;
use http_body_util::BodyExt;
use serde_json::{json, Value};
use tower::ServiceExt; // oneshot

async fn send(
    app: &Router,
    method: &str,
    uri: &str,
    token: Option<&str>,
    body: Option<Value>,
) -> (StatusCode, Value) {
    let mut builder = Request::builder().method(method).uri(uri);
    if let Some(t) = token {
        builder = builder.header("authorization", format!("Bearer {t}"));
    }
    let body = match body {
        Some(v) => {
            builder = builder.header("content-type", "application/json");
            Body::from(v.to_string())
        }
        None => Body::empty(),
    };
    let req = builder.body(body).unwrap();

    let res = app.clone().oneshot(req).await.unwrap();
    let status = res.status();
    let bytes = res.into_body().collect().await.unwrap().to_bytes();
    let json = serde_json::from_slice(&bytes).unwrap_or(Value::Null);
    (status, json)
}

fn write_index(entries: &Value) -> std::path::PathBuf {
    let path =
        std::env::temp_dir().join(format!("arrgh-plugin-index-{}.json", uuid::Uuid::new_v4()));
    std::fs::write(&path, entries.to_string()).unwrap();
    path
}

fn default_index() -> Value {
    json!([
        {
            "id": "mangadex",
            "name": "MangaDex",
            "description": "MangaDex source",
            "version": "1.0.0",
            "download_url": "http://fake-cdn/mangadex.js",
            "bundled": false,
            "default_explicit": false,
            "content_types": ["manga", "manhwa"],
        },
        {
            "id": "no-url-plugin",
            "name": "NoUrl",
            "description": null,
            "version": "1.0.0",
            "download_url": "",
            "bundled": false,
            "default_explicit": false,
            "content_types": ["manga"],
        },
    ])
}

async fn seed_plugin_source(
    state: &arrgh_server::state::AppState,
    source_key: Option<&str>,
    base_url: &str,
    is_community: bool,
) -> String {
    let id = uuid::Uuid::new_v4().to_string();
    sqlx::query(
        "INSERT INTO external_sources (id, name, base_url, content_types, enabled, created_at, is_community, priority, source_key, default_explicit) \
         VALUES (?, 'MangaDex', ?, 'manga', 1, datetime('now'), ?, 100, ?, 0)",
    )
    .bind(&id)
    .bind(base_url)
    .bind(is_community)
    .bind(source_key)
    .execute(&state.db)
    .await
    .unwrap();
    id
}

// ── GET /api/plugins/index ──────────────────────────────────────────────

#[tokio::test]
async fn index_returns_entries() {
    let path = write_index(&default_index());
    let state = common::build_plugins_state(
        &format!("file://{}", path.display()),
        "http://fake-plugin-host",
    )
    .await;
    let admin = common::seed_user(&state, "admin", "admin", true).await;
    let token = common::token_for(&admin);
    let app = arrgh_server::api::router(state);

    let (status, body) = send(&app, "GET", "/api/plugins/index", Some(&token), None).await;
    assert_eq!(status, StatusCode::OK);
    let arr = body.as_array().unwrap();
    assert_eq!(arr.len(), 2);
    assert_eq!(arr[0]["id"], "mangadex");
    assert_eq!(arr[0]["bundled"], false);

    std::fs::remove_file(&path).unwrap();
}

#[tokio::test]
async fn index_unauthorized_without_token() {
    let path = write_index(&default_index());
    let state = common::build_plugins_state(
        &format!("file://{}", path.display()),
        "http://fake-plugin-host",
    )
    .await;
    let app = arrgh_server::api::router(state);

    let (status, _) = send(&app, "GET", "/api/plugins/index", None, None).await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);

    std::fs::remove_file(&path).unwrap();
}

#[tokio::test]
async fn index_member_can_access() {
    let path = write_index(&default_index());
    let state = common::build_plugins_state(
        &format!("file://{}", path.display()),
        "http://fake-plugin-host",
    )
    .await;
    let member = common::seed_user(&state, "member1", "member", false).await;
    let token = common::token_for(&member);
    let app = arrgh_server::api::router(state);

    let (status, _) = send(&app, "GET", "/api/plugins/index", Some(&token), None).await;
    assert_eq!(status, StatusCode::OK);

    std::fs::remove_file(&path).unwrap();
}

#[tokio::test]
async fn index_bad_gateway_when_index_unreachable() {
    let state =
        common::build_plugins_state("file:///nonexistent/path.json", "http://fake-plugin-host")
            .await;
    let admin = common::seed_user(&state, "admin", "admin", true).await;
    let token = common::token_for(&admin);
    let app = arrgh_server::api::router(state);

    let (status, _) = send(&app, "GET", "/api/plugins/index", Some(&token), None).await;
    assert_eq!(status, StatusCode::BAD_GATEWAY);
}

// ── POST /api/plugins/install ───────────────────────────────────────────

#[tokio::test]
async fn install_unauthorized_without_token() {
    let path = write_index(&default_index());
    let state = common::build_plugins_state(
        &format!("file://{}", path.display()),
        "http://fake-plugin-host",
    )
    .await;
    let app = arrgh_server::api::router(state);

    let (status, _) = send(
        &app,
        "POST",
        "/api/plugins/install",
        None,
        Some(json!({"plugin_id": "mangadex"})),
    )
    .await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);

    std::fs::remove_file(&path).unwrap();
}

#[tokio::test]
async fn install_forbidden_for_member() {
    let path = write_index(&default_index());
    let state = common::build_plugins_state(
        &format!("file://{}", path.display()),
        "http://fake-plugin-host",
    )
    .await;
    let member = common::seed_user(&state, "member1", "member", false).await;
    let token = common::token_for(&member);
    let app = arrgh_server::api::router(state);

    let (status, _) = send(
        &app,
        "POST",
        "/api/plugins/install",
        Some(&token),
        Some(json!({"plugin_id": "mangadex"})),
    )
    .await;
    assert_eq!(status, StatusCode::FORBIDDEN);

    std::fs::remove_file(&path).unwrap();
}

#[tokio::test]
async fn install_bad_request_missing_plugin_id() {
    let path = write_index(&default_index());
    let state = common::build_plugins_state(
        &format!("file://{}", path.display()),
        "http://fake-plugin-host",
    )
    .await;
    let admin = common::seed_user(&state, "admin", "admin", true).await;
    let token = common::token_for(&admin);
    let app = arrgh_server::api::router(state);

    let (status, _) = send(
        &app,
        "POST",
        "/api/plugins/install",
        Some(&token),
        Some(json!({})),
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);

    std::fs::remove_file(&path).unwrap();
}

#[tokio::test]
async fn install_not_found_unknown_plugin() {
    let path = write_index(&default_index());
    let state = common::build_plugins_state(
        &format!("file://{}", path.display()),
        "http://fake-plugin-host",
    )
    .await;
    let admin = common::seed_user(&state, "admin", "admin", true).await;
    let token = common::token_for(&admin);
    let app = arrgh_server::api::router(state);

    let (status, _) = send(
        &app,
        "POST",
        "/api/plugins/install",
        Some(&token),
        Some(json!({"plugin_id": "nonexistent"})),
    )
    .await;
    assert_eq!(status, StatusCode::NOT_FOUND);

    std::fs::remove_file(&path).unwrap();
}

#[tokio::test]
async fn install_unprocessable_entity_no_download_url() {
    let path = write_index(&default_index());
    let state = common::build_plugins_state(
        &format!("file://{}", path.display()),
        "http://fake-plugin-host",
    )
    .await;
    let admin = common::seed_user(&state, "admin", "admin", true).await;
    let token = common::token_for(&admin);
    let app = arrgh_server::api::router(state);

    let (status, _) = send(
        &app,
        "POST",
        "/api/plugins/install",
        Some(&token),
        Some(json!({"plugin_id": "no-url-plugin"})),
    )
    .await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);

    std::fs::remove_file(&path).unwrap();
}

#[tokio::test]
async fn install_conflict_when_already_installed() {
    let path = write_index(&default_index());
    let plugin_host_url = "http://fake-plugin-host";
    let state =
        common::build_plugins_state(&format!("file://{}", path.display()), plugin_host_url).await;
    let admin = common::seed_user(&state, "admin", "admin", true).await;
    seed_plugin_source(&state, None, &format!("{plugin_host_url}/mangadex"), true).await;
    let token = common::token_for(&admin);
    let app = arrgh_server::api::router(state);

    let (status, _) = send(
        &app,
        "POST",
        "/api/plugins/install",
        Some(&token),
        Some(json!({"plugin_id": "mangadex"})),
    )
    .await;
    assert_eq!(status, StatusCode::CONFLICT);

    std::fs::remove_file(&path).unwrap();
}

#[tokio::test]
async fn install_bad_gateway_when_plugin_host_fails() {
    let path = write_index(&default_index());
    let mock_url = common::start_mock_plugin_host("boom", true).await;
    let state = common::build_plugins_state(&format!("file://{}", path.display()), &mock_url).await;
    let admin = common::seed_user(&state, "admin", "admin", true).await;
    let token = common::token_for(&admin);
    let app = arrgh_server::api::router(state);

    let (status, _) = send(
        &app,
        "POST",
        "/api/plugins/install",
        Some(&token),
        Some(json!({"plugin_id": "mangadex"})),
    )
    .await;
    assert_eq!(status, StatusCode::BAD_GATEWAY);

    std::fs::remove_file(&path).unwrap();
}

#[tokio::test]
async fn install_created_on_success() {
    let path = write_index(&default_index());
    let mock_url = common::start_mock_plugin_host("{}", false).await;
    let state = common::build_plugins_state(&format!("file://{}", path.display()), &mock_url).await;
    let admin = common::seed_user(&state, "admin", "admin", true).await;
    let token = common::token_for(&admin);
    let app = arrgh_server::api::router(state.clone());

    let (status, _) = send(
        &app,
        "POST",
        "/api/plugins/install",
        Some(&token),
        Some(json!({"plugin_id": "mangadex"})),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED);

    let (name, is_community, source_key): (String, bool, Option<String>) = sqlx::query_as(
        "SELECT name, is_community, source_key FROM external_sources WHERE name = 'MangaDex'",
    )
    .fetch_one(&state.db)
    .await
    .unwrap();
    assert_eq!(name, "MangaDex");
    assert!(is_community);
    // Deviation from .NET (bug fixed per user decision) — see sources.rs's
    // insert_community_source doc: installed plugins must be deletable.
    assert_eq!(source_key.as_deref(), Some("mangadex"));

    std::fs::remove_file(&path).unwrap();
}

// ── DELETE /api/plugins/{id} ─────────────────────────────────────────────

#[tokio::test]
async fn delete_unauthorized_without_token() {
    let path = write_index(&default_index());
    let state = common::build_plugins_state(
        &format!("file://{}", path.display()),
        "http://fake-plugin-host",
    )
    .await;
    let app = arrgh_server::api::router(state);

    let (status, _) = send(&app, "DELETE", "/api/plugins/mangadex", None, None).await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);

    std::fs::remove_file(&path).unwrap();
}

#[tokio::test]
async fn delete_forbidden_for_member() {
    let path = write_index(&default_index());
    let state = common::build_plugins_state(
        &format!("file://{}", path.display()),
        "http://fake-plugin-host",
    )
    .await;
    let member = common::seed_user(&state, "member1", "member", false).await;
    let token = common::token_for(&member);
    let app = arrgh_server::api::router(state);

    let (status, _) = send(&app, "DELETE", "/api/plugins/mangadex", Some(&token), None).await;
    assert_eq!(status, StatusCode::FORBIDDEN);

    std::fs::remove_file(&path).unwrap();
}

#[tokio::test]
async fn delete_not_found_unknown() {
    let path = write_index(&default_index());
    let state = common::build_plugins_state(
        &format!("file://{}", path.display()),
        "http://fake-plugin-host",
    )
    .await;
    let admin = common::seed_user(&state, "admin", "admin", true).await;
    let token = common::token_for(&admin);
    let app = arrgh_server::api::router(state);

    let (status, _) = send(
        &app,
        "DELETE",
        "/api/plugins/nonexistent",
        Some(&token),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::NOT_FOUND);

    std::fs::remove_file(&path).unwrap();
}

#[tokio::test]
async fn delete_forbidden_non_community_source() {
    let path = write_index(&default_index());
    let plugin_host_url = "http://fake-plugin-host";
    let state =
        common::build_plugins_state(&format!("file://{}", path.display()), plugin_host_url).await;
    let admin = common::seed_user(&state, "admin", "admin", true).await;
    seed_plugin_source(
        &state,
        Some("mangadex"),
        &format!("{plugin_host_url}/mangadex"),
        false,
    )
    .await;
    let token = common::token_for(&admin);
    let app = arrgh_server::api::router(state);

    let (status, _) = send(&app, "DELETE", "/api/plugins/mangadex", Some(&token), None).await;
    assert_eq!(status, StatusCode::FORBIDDEN);

    std::fs::remove_file(&path).unwrap();
}

#[tokio::test]
async fn delete_no_content_removes_source() {
    let path = write_index(&default_index());
    let plugin_host_url = "http://fake-plugin-host";
    let state =
        common::build_plugins_state(&format!("file://{}", path.display()), plugin_host_url).await;
    let admin = common::seed_user(&state, "admin", "admin", true).await;
    let source_id = seed_plugin_source(
        &state,
        Some("mangadex"),
        &format!("{plugin_host_url}/mangadex"),
        true,
    )
    .await;
    let token = common::token_for(&admin);
    let app = arrgh_server::api::router(state.clone());

    let (status, _) = send(&app, "DELETE", "/api/plugins/mangadex", Some(&token), None).await;
    assert_eq!(status, StatusCode::NO_CONTENT);

    let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM external_sources WHERE id = ?")
        .bind(&source_id)
        .fetch_one(&state.db)
        .await
        .unwrap();
    assert_eq!(count, 0);

    std::fs::remove_file(&path).unwrap();
}
