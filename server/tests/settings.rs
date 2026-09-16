//! S3 parity check for `/api/settings`. Mirrors `server-tests/SettingsTests.cs`
//! + `api-tests/tests/settings.hurl` (ADR 0033, S3 #125).

mod common;

use axum::body::Body;
use axum::http::{Request, StatusCode};
use axum::Router;
use http_body_util::BodyExt;
use serde_json::{json, Value};
use tower::ServiceExt; // oneshot

async fn send(app: &Router, method: &str, uri: &str, body: Option<Value>) -> (StatusCode, Value) {
    let builder = Request::builder().method(method).uri(uri);
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

#[tokio::test]
async fn get_settings_returns_defaults_when_nothing_saved() {
    let app = arrgh_server::api::router(common::build_state().await);
    let (_, body) = send(&app, "GET", "/api/settings", None).await;

    assert_eq!(body["download_workers"], 2);
    assert_eq!(body["index_interval_hours"], 6);
    assert_eq!(body["auto_download"], false);
    assert_eq!(body["reader_mode"], "scroll");
    assert_eq!(body["trending_per_source"], 5);
    assert_eq!(body["check_for_updates"], false);
}

#[tokio::test]
async fn get_settings_no_auth_required() {
    let app = arrgh_server::api::router(common::build_state().await);
    let (status, _) = send(&app, "GET", "/api/settings", None).await;
    assert_eq!(status, StatusCode::OK);
}

#[tokio::test]
async fn save_settings_updates_and_returns_new_values() {
    let app = arrgh_server::api::router(common::build_state().await);
    let (status, body) = send(
        &app,
        "POST",
        "/api/settings",
        Some(json!({ "download_workers": 4, "reader_mode": "scroll", "auto_download": true, "check_for_updates": true })),
    )
    .await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["download_workers"], 4);
    assert_eq!(body["reader_mode"], "scroll");
    assert_eq!(body["auto_download"], true);
    assert_eq!(body["check_for_updates"], true);
}

#[tokio::test]
async fn save_settings_partial_update_only_changes_specified_fields() {
    let app = arrgh_server::api::router(common::build_state().await);
    send(
        &app,
        "POST",
        "/api/settings",
        Some(json!({ "download_workers": 4 })),
    )
    .await;
    let (_, body) = send(&app, "GET", "/api/settings", None).await;

    assert_eq!(body["download_workers"], 4);
    assert_eq!(body["index_interval_hours"], 6);
}

#[tokio::test]
async fn save_settings_idempotent_overwrites_same_key() {
    let app = arrgh_server::api::router(common::build_state().await);
    send(
        &app,
        "POST",
        "/api/settings",
        Some(json!({ "download_workers": 4 })),
    )
    .await;
    send(
        &app,
        "POST",
        "/api/settings",
        Some(json!({ "download_workers": 8 })),
    )
    .await;
    let (_, body) = send(&app, "GET", "/api/settings", None).await;

    assert_eq!(body["download_workers"], 8);
}

#[tokio::test]
async fn save_settings_unprocessable_entity_invalid_reader_mode() {
    let app = arrgh_server::api::router(common::build_state().await);
    let (status, _) = send(
        &app,
        "POST",
        "/api/settings",
        Some(json!({ "reader_mode": "continuous" })),
    )
    .await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
}

#[tokio::test]
async fn save_settings_clamps_trending_per_source() {
    let app = arrgh_server::api::router(common::build_state().await);
    let (_, body) = send(
        &app,
        "POST",
        "/api/settings",
        Some(json!({ "trending_per_source": 999 })),
    )
    .await;
    assert_eq!(body["trending_per_source"], 50);
}

#[tokio::test]
async fn save_settings_ignores_empty_download_dir() {
    let app = arrgh_server::api::router(common::build_state().await);
    let (_, before) = send(&app, "GET", "/api/settings", None).await;
    let default_dir = before["download_dir"].as_str().unwrap().to_string();

    send(
        &app,
        "POST",
        "/api/settings",
        Some(json!({ "download_dir": "   " })),
    )
    .await;
    let (_, after) = send(&app, "GET", "/api/settings", None).await;

    assert_eq!(after["download_dir"], default_dir);
}

#[tokio::test]
async fn save_settings_trims_download_dir() {
    let app = arrgh_server::api::router(common::build_state().await);
    let (_, body) = send(
        &app,
        "POST",
        "/api/settings",
        Some(json!({ "download_dir": "  /data/downloads  " })),
    )
    .await;
    assert_eq!(body["download_dir"], "/data/downloads");
}

#[tokio::test]
async fn save_settings_no_auth_required() {
    let app = arrgh_server::api::router(common::build_state().await);
    let (status, _) = send(
        &app,
        "POST",
        "/api/settings",
        Some(json!({ "download_workers": 3 })),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
}
