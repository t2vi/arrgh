//! S2 parity check for `/api/auth/*` + `/api/users/*`. Mirrors
//! `server-tests/AuthTests.cs` + `AuthTokenTests.cs` (ADR 0033, S2 #124).

mod common;

use axum::body::Body;
use axum::http::{Request, StatusCode};
use axum::Router;
use http_body_util::BodyExt;
use serde_json::{json, Value};
use tower::ServiceExt; // oneshot

/// Sends one request against a clone of `app` and returns (status, json body).
/// Empty/non-JSON bodies decode to `Value::Null`.
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

async fn register(app: &Router, username: &str, password: &str) -> (StatusCode, Value) {
    send(
        app,
        "POST",
        "/api/auth/register",
        None,
        Some(json!({ "username": username, "password": password })),
    )
    .await
}

async fn register_and_get_token(app: &Router, username: &str, password: &str) -> String {
    let (_, body) = register(app, username, password).await;
    body["token"].as_str().unwrap().to_string()
}

// ── /api/auth/status ─────────────────────────────────────────────────────

#[tokio::test]
async fn status_needs_setup_when_no_users() {
    let app = arrgh_server::api::router(common::build_state().await);
    let (_, body) = send(&app, "GET", "/api/auth/status", None, None).await;
    assert_eq!(body["needs_setup"], true);
}

#[tokio::test]
async fn status_no_setup_needed_after_register() {
    let state = common::build_state().await;
    common::seed_user(&state, "admin", "admin", true).await;
    let app = arrgh_server::api::router(state);

    let (_, body) = send(&app, "GET", "/api/auth/status", None, None).await;
    assert_eq!(body["needs_setup"], false);
}

// ── /api/auth/register ───────────────────────────────────────────────────

#[tokio::test]
async fn register_creates_admin_and_returns_token() {
    let app = arrgh_server::api::router(common::build_state().await);
    let (status, body) = register(&app, "admin", "secret123").await;

    assert_eq!(status, StatusCode::OK);
    assert!(!body["token"].as_str().unwrap().is_empty());
    assert_eq!(body["role"], "admin");
    assert_eq!(body["allow_explicit"], true);
}

#[tokio::test]
async fn register_forbidden_when_users_exist() {
    let state = common::build_state().await;
    common::seed_user(&state, "admin", "admin", true).await;
    let app = arrgh_server::api::router(state);

    let (status, _) = register(&app, "other", "secret123").await;
    assert_eq!(status, StatusCode::FORBIDDEN);
}

#[tokio::test]
async fn register_unprocessable_entity_short_password() {
    let app = arrgh_server::api::router(common::build_state().await);
    let (status, _) = register(&app, "admin", "abc").await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
}

#[tokio::test]
async fn register_unprocessable_entity_empty_username() {
    let app = arrgh_server::api::router(common::build_state().await);
    let (status, _) = register(&app, "   ", "secret123").await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
}

// ── /api/auth/login ──────────────────────────────────────────────────────

#[tokio::test]
async fn login_returns_token_valid_credentials() {
    let app = arrgh_server::api::router(common::build_state().await);
    register(&app, "admin", "secret123").await;

    let (status, body) = send(
        &app,
        "POST",
        "/api/auth/login",
        None,
        Some(json!({ "username": "admin", "password": "secret123" })),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert!(!body["token"].as_str().unwrap().is_empty());
}

#[tokio::test]
async fn login_unauthorized_wrong_password() {
    let app = arrgh_server::api::router(common::build_state().await);
    register(&app, "admin", "secret123").await;

    let (status, _) = send(
        &app,
        "POST",
        "/api/auth/login",
        None,
        Some(json!({ "username": "admin", "password": "wrong" })),
    )
    .await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn login_unauthorized_unknown_user() {
    let app = arrgh_server::api::router(common::build_state().await);
    let (status, _) = send(
        &app,
        "POST",
        "/api/auth/login",
        None,
        Some(json!({ "username": "ghost", "password": "secret123" })),
    )
    .await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
}

// ── /api/auth/me ─────────────────────────────────────────────────────────

#[tokio::test]
async fn me_returns_current_user() {
    let app = arrgh_server::api::router(common::build_state().await);
    let token = register_and_get_token(&app, "admin", "secret123").await;

    let (_, body) = send(&app, "GET", "/api/auth/me", Some(&token), None).await;
    assert_eq!(body["username"], "admin");
    assert_eq!(body["role"], "admin");
}

#[tokio::test]
async fn me_unauthorized_no_token() {
    let app = arrgh_server::api::router(common::build_state().await);
    let (status, _) = send(&app, "GET", "/api/auth/me", None, None).await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
}

// ── /api/users ───────────────────────────────────────────────────────────

#[tokio::test]
async fn list_users_forbidden_for_member() {
    let state = common::build_state().await;
    let member = common::seed_user(&state, "member1", "member", false).await;
    let token = common::token_for(&member);
    let app = arrgh_server::api::router(state);

    let (status, _) = send(&app, "GET", "/api/users", Some(&token), None).await;
    assert_eq!(status, StatusCode::FORBIDDEN);
}

#[tokio::test]
async fn create_user_conflict_duplicate_username() {
    let app = arrgh_server::api::router(common::build_state().await);
    let token = register_and_get_token(&app, "admin", "secret123").await;

    send(
        &app,
        "POST",
        "/api/users",
        Some(&token),
        Some(json!({ "username": "bob", "password": "secret123" })),
    )
    .await;
    let (status, _) = send(
        &app,
        "POST",
        "/api/users",
        Some(&token),
        Some(json!({ "username": "bob", "password": "secret123" })),
    )
    .await;
    assert_eq!(status, StatusCode::CONFLICT);
}

#[tokio::test]
async fn delete_user_forbidden_cannot_delete_self() {
    let app = arrgh_server::api::router(common::build_state().await);
    let (_, body) = register(&app, "admin", "secret123").await;
    let user_id = body["user_id"].as_str().unwrap();
    let token = body["token"].as_str().unwrap();

    let (status, _) = send(
        &app,
        "DELETE",
        &format!("/api/users/{user_id}"),
        Some(token),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::FORBIDDEN);
}

// ── /api/auth/me PATCH ───────────────────────────────────────────────────

#[tokio::test]
async fn patch_me_changes_password() {
    let app = arrgh_server::api::router(common::build_state().await);
    let token = register_and_get_token(&app, "admin", "oldpass1").await;

    let (status, _) = send(
        &app,
        "PATCH",
        "/api/auth/me",
        Some(&token),
        Some(json!({ "password": "newpass1" })),
    )
    .await;
    assert_eq!(status, StatusCode::OK);

    let (old_status, _) = send(
        &app,
        "POST",
        "/api/auth/login",
        None,
        Some(json!({ "username": "admin", "password": "oldpass1" })),
    )
    .await;
    assert_eq!(old_status, StatusCode::UNAUTHORIZED);

    let (new_status, _) = send(
        &app,
        "POST",
        "/api/auth/login",
        None,
        Some(json!({ "username": "admin", "password": "newpass1" })),
    )
    .await;
    assert_eq!(new_status, StatusCode::OK);
}

#[tokio::test]
async fn patch_me_unprocessable_entity_short_password() {
    let app = arrgh_server::api::router(common::build_state().await);
    let token = register_and_get_token(&app, "admin", "secret123").await;

    let (status, _) = send(
        &app,
        "PATCH",
        "/api/auth/me",
        Some(&token),
        Some(json!({ "password": "abc" })),
    )
    .await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
}

// ── /api/users full CRUD ─────────────────────────────────────────────────

#[tokio::test]
async fn list_users_returns_all_users_for_admin() {
    let app = arrgh_server::api::router(common::build_state().await);
    let token = register_and_get_token(&app, "admin", "secret123").await;

    send(
        &app,
        "POST",
        "/api/users",
        Some(&token),
        Some(json!({ "username": "bob", "password": "secret123" })),
    )
    .await;
    send(
        &app,
        "POST",
        "/api/users",
        Some(&token),
        Some(json!({ "username": "carol", "password": "secret123" })),
    )
    .await;

    let (_, body) = send(&app, "GET", "/api/users", Some(&token), None).await;
    assert_eq!(body.as_array().unwrap().len(), 3);
}

#[tokio::test]
async fn create_user_created_valid_member() {
    let app = arrgh_server::api::router(common::build_state().await);
    let token = register_and_get_token(&app, "admin", "secret123").await;

    let (status, _) = send(
        &app,
        "POST",
        "/api/users",
        Some(&token),
        Some(json!({ "username": "bob", "password": "secret123" })),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED);

    let (login_status, body) = send(
        &app,
        "POST",
        "/api/auth/login",
        None,
        Some(json!({ "username": "bob", "password": "secret123" })),
    )
    .await;
    assert_eq!(login_status, StatusCode::OK);
    assert_eq!(body["role"], "member");
    assert_eq!(body["allow_explicit"], false);
}

#[tokio::test]
async fn create_user_forbidden_for_member() {
    let state = common::build_state().await;
    let member = common::seed_user(&state, "member1", "member", false).await;
    let token = common::token_for(&member);
    let app = arrgh_server::api::router(state);

    let (status, _) = send(
        &app,
        "POST",
        "/api/users",
        Some(&token),
        Some(json!({ "username": "x", "password": "secret123" })),
    )
    .await;
    assert_eq!(status, StatusCode::FORBIDDEN);
}

async fn user_id_by_username(app: &Router, token: &str, username: &str) -> String {
    let (_, body) = send(app, "GET", "/api/users", Some(token), None).await;
    body.as_array()
        .unwrap()
        .iter()
        .find(|u| u["username"] == username)
        .unwrap()["id"]
        .as_str()
        .unwrap()
        .to_string()
}

#[tokio::test]
async fn patch_user_updates_role() {
    let app = arrgh_server::api::router(common::build_state().await);
    let token = register_and_get_token(&app, "admin", "secret123").await;

    let (create_status, _) = send(
        &app,
        "POST",
        "/api/users",
        Some(&token),
        Some(json!({ "username": "bob", "password": "secret123" })),
    )
    .await;
    assert_eq!(create_status, StatusCode::CREATED);
    let bob_id = user_id_by_username(&app, &token, "bob").await;

    let (patch_status, _) = send(
        &app,
        "PATCH",
        &format!("/api/users/{bob_id}"),
        Some(&token),
        Some(json!({ "role": "admin" })),
    )
    .await;
    assert_eq!(patch_status, StatusCode::NO_CONTENT);

    let (_, body) = send(&app, "GET", "/api/users", Some(&token), None).await;
    let bob = body
        .as_array()
        .unwrap()
        .iter()
        .find(|u| u["id"] == bob_id)
        .unwrap();
    assert_eq!(bob["role"], "admin");
}

#[tokio::test]
async fn patch_user_updates_allow_explicit() {
    let app = arrgh_server::api::router(common::build_state().await);
    let token = register_and_get_token(&app, "admin", "secret123").await;

    send(
        &app,
        "POST",
        "/api/users",
        Some(&token),
        Some(json!({ "username": "bob", "password": "secret123" })),
    )
    .await;
    let bob_id = user_id_by_username(&app, &token, "bob").await;

    let (_, body) = send(&app, "GET", "/api/users", Some(&token), None).await;
    let bob = body
        .as_array()
        .unwrap()
        .iter()
        .find(|u| u["id"] == bob_id)
        .unwrap();
    assert_eq!(bob["allow_explicit"], false);

    send(
        &app,
        "PATCH",
        &format!("/api/users/{bob_id}"),
        Some(&token),
        Some(json!({ "allow_explicit": true })),
    )
    .await;
    let (_, body) = send(&app, "GET", "/api/users", Some(&token), None).await;
    let bob = body
        .as_array()
        .unwrap()
        .iter()
        .find(|u| u["id"] == bob_id)
        .unwrap();
    assert_eq!(bob["allow_explicit"], true);
}

#[tokio::test]
async fn patch_user_unprocessable_entity_invalid_role() {
    let app = arrgh_server::api::router(common::build_state().await);
    let token = register_and_get_token(&app, "admin", "secret123").await;

    send(
        &app,
        "POST",
        "/api/users",
        Some(&token),
        Some(json!({ "username": "bob", "password": "secret123" })),
    )
    .await;
    let bob_id = user_id_by_username(&app, &token, "bob").await;

    let (status, _) = send(
        &app,
        "PATCH",
        &format!("/api/users/{bob_id}"),
        Some(&token),
        Some(json!({ "role": "superuser" })),
    )
    .await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
}

#[tokio::test]
async fn patch_user_not_found_nonexistent_user() {
    let app = arrgh_server::api::router(common::build_state().await);
    let token = register_and_get_token(&app, "admin", "secret123").await;

    let (status, _) = send(
        &app,
        "PATCH",
        "/api/users/ghost",
        Some(&token),
        Some(json!({ "role": "member" })),
    )
    .await;
    assert_eq!(status, StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn delete_user_no_content_success() {
    let app = arrgh_server::api::router(common::build_state().await);
    let token = register_and_get_token(&app, "admin", "secret123").await;

    send(
        &app,
        "POST",
        "/api/users",
        Some(&token),
        Some(json!({ "username": "bob", "password": "secret123" })),
    )
    .await;
    let bob_id = user_id_by_username(&app, &token, "bob").await;

    let (status, _) = send(
        &app,
        "DELETE",
        &format!("/api/users/{bob_id}"),
        Some(&token),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::NO_CONTENT);

    let (_, body) = send(&app, "GET", "/api/users", Some(&token), None).await;
    assert_eq!(body.as_array().unwrap().len(), 1);
}

#[tokio::test]
async fn delete_user_not_found_nonexistent_user() {
    let app = arrgh_server::api::router(common::build_state().await);
    let token = register_and_get_token(&app, "admin", "secret123").await;

    let (status, _) = send(&app, "DELETE", "/api/users/ghost", Some(&token), None).await;
    assert_eq!(status, StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn delete_user_forbidden_for_member() {
    let state = common::build_state().await;
    let admin = common::seed_user(&state, "admin1", "admin", true).await;
    let member = common::seed_user(&state, "member1", "member", false).await;
    let token = common::token_for(&member);
    let app = arrgh_server::api::router(state);

    let (status, _) = send(
        &app,
        "DELETE",
        &format!("/api/users/{}", admin.id),
        Some(&token),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::FORBIDDEN);
}
