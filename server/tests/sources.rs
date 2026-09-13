//! S3 parity check for `/api/sources`. Mirrors `server-tests/SourcesTests.cs`
//! plus `api-tests/tests/sources.hurl` (ADR 0033, S3 #125). Two .NET test
//! classes are deliberately not ported: `DefaultSeedingTests` covers
//! `Program.cs`'s own seeding logic, and `SourcesLogicTests` covers an
//! `IsAdmin` helper — both live on the .NET side only.

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
    let req = if let Some(b) = body {
        builder
            .header("content-type", "application/json")
            .body(Body::from(serde_json::to_vec(&b).unwrap()))
            .unwrap()
    } else {
        builder.body(Body::empty()).unwrap()
    };

    let res = app.clone().oneshot(req).await.unwrap();
    let status = res.status();
    let bytes = res.into_body().collect().await.unwrap().to_bytes();
    let json = serde_json::from_slice(&bytes).unwrap_or(Value::Null);
    (status, json)
}

// ── GET /api/sources ─────────────────────────────────────────────────────

#[tokio::test]
async fn list_sources_returns_empty_when_none() {
    let state = common::build_state().await;
    let admin = common::seed_user(&state, "admin", "admin", true).await;
    let token = common::token_for(&admin);
    let app = arrgh_server::api::router(state);

    let (_, body) = send(&app, "GET", "/api/sources", Some(&token), None).await;
    assert_eq!(body.as_array().unwrap().len(), 0);
}

#[tokio::test]
async fn list_sources_returns_sources_with_content_types_array() {
    let state = common::build_state().await;
    let admin = common::seed_user(&state, "admin", "admin", true).await;
    common::seed_source(
        &state,
        "MangaDex",
        "http://mangadex.org",
        "manga,manhwa,manhua",
        None,
    )
    .await;
    let token = common::token_for(&admin);
    let app = arrgh_server::api::router(state);

    let (_, body) = send(&app, "GET", "/api/sources", Some(&token), None).await;
    let arr = body.as_array().unwrap();
    assert_eq!(arr.len(), 1);
    let src = &arr[0];
    assert_eq!(src["name"], "MangaDex");
    assert_eq!(src["has_api_key"], false);
    let ct: Vec<&str> = src["content_types"]
        .as_array()
        .unwrap()
        .iter()
        .map(|v| v.as_str().unwrap())
        .collect();
    assert!(ct.contains(&"manga"));
    assert!(ct.contains(&"manhwa"));
    assert!(ct.contains(&"manhua"));
}

#[tokio::test]
async fn list_sources_has_api_key_true_when_api_key_set() {
    let state = common::build_state().await;
    let admin = common::seed_user(&state, "admin", "admin", true).await;
    common::seed_source(
        &state,
        "Private",
        "http://private.example.com",
        "manga",
        Some("secret-key"),
    )
    .await;
    let token = common::token_for(&admin);
    let app = arrgh_server::api::router(state);

    let (_, body) = send(&app, "GET", "/api/sources", Some(&token), None).await;
    assert_eq!(body[0]["has_api_key"], true);
}

#[tokio::test]
async fn list_sources_unauthorized_no_token() {
    let app = arrgh_server::api::router(common::build_state().await);
    let (status, _) = send(&app, "GET", "/api/sources", None, None).await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
}

// ── POST /api/sources ────────────────────────────────────────────────────

#[tokio::test]
async fn add_source_forbidden_for_member() {
    let state = common::build_state().await;
    let member = common::seed_user(&state, "member1", "member", false).await;
    let token = common::token_for(&member);
    let app = arrgh_server::api::router(state);

    let (status, _) = send(
        &app,
        "POST",
        "/api/sources",
        Some(&token),
        Some(json!({ "base_url": "http://example.com" })),
    )
    .await;
    assert_eq!(status, StatusCode::FORBIDDEN);
}

#[tokio::test]
async fn add_source_bad_gateway_when_plugin_not_ported_yet() {
    let state = common::build_state().await;
    let admin = common::seed_user(&state, "admin", "admin", true).await;
    let token = common::token_for(&admin);
    let app = arrgh_server::api::router(state);

    let (status, _) = send(
        &app,
        "POST",
        "/api/sources",
        Some(&token),
        Some(json!({ "base_url": "http://example.com" })),
    )
    .await;
    assert_eq!(status, StatusCode::BAD_GATEWAY);
}

// ── PATCH /api/sources/{id} ──────────────────────────────────────────────

#[tokio::test]
async fn patch_source_toggles_enabled() {
    let state = common::build_state().await;
    let admin = common::seed_user(&state, "admin", "admin", true).await;
    let source_id = common::seed_source(&state, "Test", "http://test.com", "manga", None).await;
    let token = common::token_for(&admin);
    let app = arrgh_server::api::router(state);

    let (status, _) = send(
        &app,
        "PATCH",
        &format!("/api/sources/{source_id}"),
        Some(&token),
        Some(json!({ "enabled": false })),
    )
    .await;
    assert_eq!(status, StatusCode::NO_CONTENT);

    let (_, body) = send(&app, "GET", "/api/sources", Some(&token), None).await;
    assert_eq!(body[0]["enabled"], false);
}

#[tokio::test]
async fn patch_source_not_found_nonexistent_id() {
    let state = common::build_state().await;
    let admin = common::seed_user(&state, "admin", "admin", true).await;
    let token = common::token_for(&admin);
    let app = arrgh_server::api::router(state);

    let (status, _) = send(
        &app,
        "PATCH",
        "/api/sources/ghost",
        Some(&token),
        Some(json!({ "enabled": false })),
    )
    .await;
    assert_eq!(status, StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn patch_source_forbidden_for_member() {
    let state = common::build_state().await;
    let member = common::seed_user(&state, "member1", "member", false).await;
    let token = common::token_for(&member);
    let app = arrgh_server::api::router(state);

    let (status, _) = send(
        &app,
        "PATCH",
        "/api/sources/any",
        Some(&token),
        Some(json!({ "enabled": false })),
    )
    .await;
    assert_eq!(status, StatusCode::FORBIDDEN);
}

#[tokio::test]
async fn patch_source_can_update_priority() {
    let state = common::build_state().await;
    let admin = common::seed_user(&state, "admin", "admin", true).await;
    let source_id = common::seed_source(&state, "Test", "http://test.com", "manhwa", None).await;
    let token = common::token_for(&admin);
    let app = arrgh_server::api::router(state);

    let (status, _) = send(
        &app,
        "PATCH",
        &format!("/api/sources/{source_id}"),
        Some(&token),
        Some(json!({ "enabled": true, "priority": 110 })),
    )
    .await;
    assert_eq!(status, StatusCode::NO_CONTENT);

    let (_, body) = send(&app, "GET", "/api/sources", Some(&token), None).await;
    assert_eq!(body[0]["priority"], 110);
}

// ── DELETE /api/sources/{id} ─────────────────────────────────────────────

#[tokio::test]
async fn delete_source_no_content_when_exists() {
    let state = common::build_state().await;
    let admin = common::seed_user(&state, "admin", "admin", true).await;
    let source_id = common::seed_source(&state, "Test", "http://test.com", "manga", None).await;
    let token = common::token_for(&admin);
    let app = arrgh_server::api::router(state);

    let (status, _) = send(
        &app,
        "DELETE",
        &format!("/api/sources/{source_id}"),
        Some(&token),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::NO_CONTENT);

    let (_, body) = send(&app, "GET", "/api/sources", Some(&token), None).await;
    assert_eq!(body.as_array().unwrap().len(), 0);
}

#[tokio::test]
async fn delete_source_not_found_nonexistent_id() {
    let state = common::build_state().await;
    let admin = common::seed_user(&state, "admin", "admin", true).await;
    let token = common::token_for(&admin);
    let app = arrgh_server::api::router(state);

    let (status, _) = send(&app, "DELETE", "/api/sources/ghost", Some(&token), None).await;
    assert_eq!(status, StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn delete_source_forbidden_for_member() {
    let state = common::build_state().await;
    let member = common::seed_user(&state, "member1", "member", false).await;
    let token = common::token_for(&member);
    let app = arrgh_server::api::router(state);

    let (status, _) = send(&app, "DELETE", "/api/sources/any", Some(&token), None).await;
    assert_eq!(status, StatusCode::FORBIDDEN);
}
