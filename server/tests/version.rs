//! S0 parity check for `GET /api/version`. Mirrors `api-tests/version.hurl`.

use axum::body::Body;
use axum::http::{Request, StatusCode};
use http_body_util::BodyExt;
use tower::ServiceExt; // oneshot

use arrgh_server::config::Config;
use arrgh_server::state::AppState;

fn test_state() -> AppState {
    // Config::from_env with nothing set → all defaults, no panic.
    AppState::new(Config::from_env().expect("default config"))
}

#[tokio::test]
async fn version_returns_current_and_no_update() {
    let app = arrgh_server::api::router(test_state());

    let res = app
        .oneshot(Request::get("/api/version").body(Body::empty()).unwrap())
        .await
        .unwrap();

    assert_eq!(res.status(), StatusCode::OK);

    let bytes = res.into_body().collect().await.unwrap().to_bytes();
    let body: serde_json::Value = serde_json::from_slice(&bytes).unwrap();

    assert_eq!(body["current"], env!("CARGO_PKG_VERSION"));
    assert!(body["latest"].is_null());
    assert!(body["release_url"].is_null());
}

#[tokio::test]
async fn version_reports_update_when_cache_has_newer() {
    let state = test_state();
    state
        .update
        .set("9.9.9", "https://example.test/releases/9.9.9");
    let app = arrgh_server::api::router(state);

    let res = app
        .oneshot(Request::get("/api/version").body(Body::empty()).unwrap())
        .await
        .unwrap();

    let bytes = res.into_body().collect().await.unwrap().to_bytes();
    let body: serde_json::Value = serde_json::from_slice(&bytes).unwrap();

    assert_eq!(body["current"], env!("CARGO_PKG_VERSION"));
    assert_eq!(body["latest"], "9.9.9");
    assert_eq!(body["release_url"], "https://example.test/releases/9.9.9");
}
