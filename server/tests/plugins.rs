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
            "sha256": "abc123",
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

// ── spec 031 phase A: catalog fallback, status, update, revert ─────────────

async fn admin_app(state: arrgh_server::state::AppState) -> (Router, String) {
    let admin = common::seed_user(&state, "admin", "admin", true).await;
    (arrgh_server::api::router(state), common::token_for(&admin))
}

async fn member_token(state: &arrgh_server::state::AppState) -> String {
    common::token_for(&common::seed_user(state, "member1", "member", false).await)
}

/// Catalog for update tests: `p` 1.1.0 is updatable; the others are each blocked one way.
fn update_index() -> Value {
    let entry = |id: &str, url: Value, sha: Value, protocol: u32| {
        json!({
            "id": id, "name": id, "version": "1.1.0", "download_url": url, "sha256": sha,
            "protocol": protocol, "bundled": true, "default_explicit": false, "content_types": ["manga"],
        })
    };
    json!([
        entry("p", json!("https://cdn.example/p.js"), json!("abc123"), 1),
        entry("nourl", json!(null), json!("abc123"), 1),
        entry(
            "nosha",
            json!("https://cdn.example/nosha.js"),
            json!(null),
            1
        ),
        entry(
            "future",
            json!("https://cdn.example/future.js"),
            json!("abc123"),
            2
        ),
    ])
}

const HOST_PLUGINS: &str = r#"[
  {"id":"p","name":"P","version":"1.0.0","origin":"bundled","has_bundled":true,"content_types":["manga"]},
  {"id":"nourl","name":"NoUrl","version":"1.0.0","origin":"bundled","has_bundled":true,"content_types":["manga"]},
  {"id":"nosha","name":"NoSha","version":null,"origin":"bundled","has_bundled":true,"content_types":["manga"]},
  {"id":"future","name":"Future","version":"1.0.0","origin":"downloaded","has_bundled":true,"content_types":["manga"]},
  {"id":"local","name":"Local","version":"3.0.0","origin":"downloaded","has_bundled":false,"content_types":["manga"]}
]"#;

// spec: 031/FR-001, 031/FR-002
#[tokio::test]
async fn index_falls_back_to_the_image_copy_when_live_fails() {
    let fallback = write_index(&update_index());
    let state = common::build_plugins_state_with_fallback(
        "http://127.0.0.1:9/unreachable.json",
        &format!("file://{}", fallback.display()),
        "http://fake-plugin-host",
    )
    .await;
    let (app, token) = admin_app(state).await;

    let (status, body) = send(&app, "GET", "/api/plugins/index", Some(&token), None).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body[0]["sha256"], "abc123");
    assert_eq!(body[3]["protocol"], 2);
    std::fs::remove_file(&fallback).unwrap();
}

// spec: 031/FR-003, 031/FR-006, 031/FR-010, 031/FR-011
#[tokio::test]
async fn status_merges_loaded_plugins_with_the_catalog() {
    let path = write_index(&update_index());
    let (host, seen) = common::start_recording_mock(&[
        ("GET /plugins", 200, HOST_PLUGINS),
        ("GET /host", 200, r#"{"protocol":1}"#),
    ])
    .await;
    let state = common::build_plugins_state(&format!("file://{}", path.display()), &host).await;
    let (app, token) = admin_app(state).await;

    let (status, body) = send(&app, "GET", "/api/plugins", Some(&token), None).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["catalog"], "live");
    assert_eq!(body["host_protocol"], 1);
    let row = |id: &str| {
        body["plugins"]
            .as_array()
            .unwrap()
            .iter()
            .find(|p| p["id"] == id)
            .unwrap()
            .clone()
    };

    let p = row("p");
    assert_eq!(p["loaded_version"], "1.0.0");
    assert_eq!(p["origin"], "bundled");
    assert_eq!(p["catalog_version"], "1.1.0");
    assert_eq!(p["update_available"], true);
    assert_eq!(p["blocked_reason"], Value::Null);

    assert!(row("nourl")["blocked_reason"]
        .as_str()
        .unwrap()
        .contains("download"));
    // unknown loaded version → update still offered (US3 AS2)
    assert_eq!(row("nosha")["update_available"], true);
    assert!(row("nosha")["blocked_reason"]
        .as_str()
        .unwrap()
        .contains("checksum"));
    assert!(row("future")["blocked_reason"]
        .as_str()
        .unwrap()
        .contains("newer *ARRgh"));

    let local = row("local");
    assert_eq!(local["catalog_version"], Value::Null);
    assert_eq!(local["update_available"], false);
    assert_eq!(local["has_bundled"], false);

    // Listing never installs anything on its own (FR-011).
    assert!(seen.lock().unwrap().iter().all(|(m, _, _)| m == "GET"));
    std::fs::remove_file(&path).unwrap();
}

// spec: 031/FR-002
#[tokio::test]
async fn status_reports_fallback_catalog_and_survives_no_catalog() {
    let fallback = write_index(&update_index());
    let (host, _) = common::start_recording_mock(&[
        ("GET /plugins", 200, HOST_PLUGINS),
        ("GET /host", 200, r#"{"protocol":1}"#),
    ])
    .await;
    let state = common::build_plugins_state_with_fallback(
        "http://127.0.0.1:9/unreachable.json",
        &format!("file://{}", fallback.display()),
        &host,
    )
    .await;
    let (app, token) = admin_app(state).await;
    let (_, body) = send(&app, "GET", "/api/plugins", Some(&token), None).await;
    assert_eq!(body["catalog"], "fallback");

    let state = common::build_plugins_state("http://127.0.0.1:9/unreachable.json", &host).await;
    let (app, token) = admin_app(state).await;
    let (status, body) = send(&app, "GET", "/api/plugins", Some(&token), None).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["catalog"], Value::Null);
    assert_eq!(body["plugins"].as_array().unwrap().len(), 5);
    std::fs::remove_file(&fallback).unwrap();
}

#[tokio::test]
async fn status_bad_gateway_when_plugin_host_is_down() {
    let path = write_index(&update_index());
    let state =
        common::build_plugins_state(&format!("file://{}", path.display()), "http://127.0.0.1:9")
            .await;
    let (app, token) = admin_app(state).await;
    let (status, _) = send(&app, "GET", "/api/plugins", Some(&token), None).await;
    assert_eq!(status, StatusCode::BAD_GATEWAY);
    std::fs::remove_file(&path).unwrap();
}

// spec: 031/FR-004
#[tokio::test]
async fn plugin_admin_routes_are_admin_only() {
    let path = write_index(&update_index());
    let state = common::build_plugins_state(
        &format!("file://{}", path.display()),
        "http://fake-plugin-host",
    )
    .await;
    let member = member_token(&state).await;
    let app = arrgh_server::api::router(state);
    for (method, uri) in [
        ("GET", "/api/plugins"),
        ("POST", "/api/plugins/p/update"),
        ("POST", "/api/plugins/p/revert"),
    ] {
        assert_eq!(
            send(&app, method, uri, None, None).await.0,
            StatusCode::UNAUTHORIZED,
            "{uri}"
        );
        assert_eq!(
            send(&app, method, uri, Some(&member), None).await.0,
            StatusCode::FORBIDDEN,
            "{uri}"
        );
    }
    std::fs::remove_file(&path).unwrap();
}

// spec: 031/FR-005, 031/FR-006
#[tokio::test]
async fn update_forwards_the_catalog_entry_to_the_host() {
    let path = write_index(&update_index());
    let (host, seen) = common::start_recording_mock(&[
        ("GET /host", 200, r#"{"protocol":1}"#),
        (
            "POST /plugins/install",
            201,
            r#"{"id":"p","version":"1.1.0","origin":"downloaded"}"#,
        ),
    ])
    .await;
    let state = common::build_plugins_state(&format!("file://{}", path.display()), &host).await;
    let (app, token) = admin_app(state).await;

    let (status, body) = send(&app, "POST", "/api/plugins/p/update", Some(&token), None).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(
        body,
        json!({"id":"p","version":"1.1.0","origin":"downloaded"})
    );
    let install = seen
        .lock()
        .unwrap()
        .iter()
        .find(|(m, _, _)| m == "POST")
        .unwrap()
        .2
        .clone();
    assert_eq!(
        install,
        json!({"id":"p","url":"https://cdn.example/p.js","sha256":"abc123","protocol":1})
    );
    std::fs::remove_file(&path).unwrap();
}

// spec: 031/FR-005, 031/FR-006, 031/FR-012
#[tokio::test]
async fn update_refuses_blocked_entries_without_calling_install() {
    let path = write_index(&update_index());
    let (host, seen) =
        common::start_recording_mock(&[("GET /host", 200, r#"{"protocol":1}"#)]).await;
    let state = common::build_plugins_state(&format!("file://{}", path.display()), &host).await;
    let (app, token) = admin_app(state).await;

    for (id, why) in [
        ("nourl", "download"),
        ("nosha", "checksum"),
        ("future", "newer *ARRgh"),
    ] {
        let (status, body) = send(
            &app,
            "POST",
            &format!("/api/plugins/{id}/update"),
            Some(&token),
            None,
        )
        .await;
        assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY, "{id}");
        assert!(
            body["error"].as_str().unwrap().contains(why),
            "{id}: {body}"
        );
    }
    let (status, _) = send(
        &app,
        "POST",
        "/api/plugins/unknown/update",
        Some(&token),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::NOT_FOUND);
    assert!(seen.lock().unwrap().iter().all(|(m, _, _)| m == "GET"));
    std::fs::remove_file(&path).unwrap();
}

// spec: 031/FR-012
#[tokio::test]
async fn update_surfaces_the_hosts_rejection() {
    let path = write_index(&update_index());
    let (host, _) = common::start_recording_mock(&[
        ("GET /host", 200, r#"{"protocol":1}"#),
        (
            "POST /plugins/install",
            422,
            r#"{"error":"checksum mismatch: expected abc123, got def"}"#,
        ),
    ])
    .await;
    let state = common::build_plugins_state(&format!("file://{}", path.display()), &host).await;
    let (app, token) = admin_app(state).await;
    let (status, body) = send(&app, "POST", "/api/plugins/p/update", Some(&token), None).await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
    assert!(body["error"]
        .as_str()
        .unwrap()
        .contains("checksum mismatch"));

    let (host, _) = common::start_recording_mock(&[
        ("GET /host", 200, r#"{"protocol":1}"#),
        (
            "POST /plugins/install",
            502,
            r#"{"error":"download failed: 404"}"#,
        ),
    ])
    .await;
    let state = common::build_plugins_state(&format!("file://{}", path.display()), &host).await;
    let (app, token) = admin_app(state).await;
    let (status, body) = send(&app, "POST", "/api/plugins/p/update", Some(&token), None).await;
    assert_eq!(status, StatusCode::BAD_GATEWAY);
    assert!(body["error"].as_str().unwrap().contains("download failed"));
    std::fs::remove_file(&path).unwrap();
}

// spec: 031/FR-005
#[tokio::test]
async fn install_refuses_an_entry_without_checksum() {
    let path = write_index(&update_index());
    let (host, seen) = common::start_recording_mock(&[]).await;
    let state = common::build_plugins_state(&format!("file://{}", path.display()), &host).await;
    let (app, token) = admin_app(state).await;
    let (status, body) = send(
        &app,
        "POST",
        "/api/plugins/install",
        Some(&token),
        Some(json!({"plugin_id": "nosha"})),
    )
    .await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
    assert!(body["error"].as_str().unwrap().contains("checksum"));
    assert!(seen.lock().unwrap().is_empty());
    std::fs::remove_file(&path).unwrap();
}

// spec: 031/FR-005
#[tokio::test]
async fn install_forwards_id_and_checksum() {
    let path = write_index(&default_index());
    let (host, seen) =
        common::start_recording_mock(&[("POST /plugins/install", 201, r#"{"id":"mangadex"}"#)])
            .await;
    let state = common::build_plugins_state(&format!("file://{}", path.display()), &host).await;
    let (app, token) = admin_app(state).await;
    let (status, _) = send(
        &app,
        "POST",
        "/api/plugins/install",
        Some(&token),
        Some(json!({"plugin_id": "mangadex"})),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED);
    assert_eq!(
        seen.lock().unwrap()[0].2,
        json!({"id":"mangadex","url":"http://fake-cdn/mangadex.js","sha256":"abc123","protocol":1})
    );
    std::fs::remove_file(&path).unwrap();
}

// spec: 031/FR-009
#[tokio::test]
async fn revert_passes_through_and_maps_nothing_to_revert() {
    let path = write_index(&update_index());
    let (host, seen) = common::start_recording_mock(&[
        (
            "DELETE /plugins/p",
            200,
            r#"{"id":"p","version":"1.0.0","origin":"bundled"}"#,
        ),
        (
            "DELETE /plugins/nourl",
            403,
            r#"{"error":"nothing downloaded to remove"}"#,
        ),
    ])
    .await;
    let state = common::build_plugins_state(&format!("file://{}", path.display()), &host).await;
    let (app, token) = admin_app(state).await;

    let (status, body) = send(&app, "POST", "/api/plugins/p/revert", Some(&token), None).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["origin"], "bundled");
    assert_eq!(seen.lock().unwrap()[0].0, "DELETE");

    let (status, _) = send(
        &app,
        "POST",
        "/api/plugins/nourl/revert",
        Some(&token),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::CONFLICT);
    let (status, _) = send(
        &app,
        "POST",
        "/api/plugins/ghost/revert",
        Some(&token),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::NOT_FOUND);
    std::fs::remove_file(&path).unwrap();
}
