//! S1 parity check for `/api/logs`. Mirrors `api-tests/tests/logs.hurl`.

mod common;

use axum::body::Body;
use axum::http::{Request, StatusCode};
use http_body_util::BodyExt;
use tower::ServiceExt; // oneshot

fn token(role: &str) -> String {
    arrgh_server::auth::create_token("user-1", "tester", role, false, common::JWT_SECRET).unwrap()
}

#[tokio::test]
async fn list_requires_auth() {
    let app = arrgh_server::api::router(common::build_state().await);
    let res = app
        .oneshot(Request::get("/api/logs").body(Body::empty()).unwrap())
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn list_returns_array_with_valid_token() {
    let app = arrgh_server::api::router(common::build_state().await);
    let res = app
        .oneshot(
            Request::get("/api/logs")
                .header("authorization", format!("Bearer {}", token("member")))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(res.status(), StatusCode::OK);
    let bytes = res.into_body().collect().await.unwrap().to_bytes();
    let body: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    assert!(body.is_array());
}

#[tokio::test]
async fn set_level_requires_admin() {
    let app = arrgh_server::api::router(common::build_state().await);
    let res = app
        .oneshot(
            Request::patch("/api/logs/level")
                .header("authorization", format!("Bearer {}", token("member")))
                .header("content-type", "application/json")
                .body(Body::from(r#"{"level":"debug"}"#))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(res.status(), StatusCode::FORBIDDEN);
}

#[tokio::test]
async fn set_level_admin_roundtrip() {
    let app = arrgh_server::api::router(common::build_state().await);

    let res = app
        .clone()
        .oneshot(
            Request::patch("/api/logs/level")
                .header("authorization", format!("Bearer {}", token("admin")))
                .header("content-type", "application/json")
                .body(Body::from(r#"{"level":"debug"}"#))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::NO_CONTENT);

    let res = app
        .oneshot(
            Request::get("/api/logs/level")
                .header("authorization", format!("Bearer {}", token("admin")))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    let bytes = res.into_body().collect().await.unwrap().to_bytes();
    let body: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(body["level"], "DEBUG");
}

#[tokio::test]
async fn set_level_rejects_unknown_value() {
    let app = arrgh_server::api::router(common::build_state().await);
    let res = app
        .oneshot(
            Request::patch("/api/logs/level")
                .header("authorization", format!("Bearer {}", token("admin")))
                .header("content-type", "application/json")
                .body(Body::from(r#"{"level":"nonsense"}"#))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(res.status(), StatusCode::UNPROCESSABLE_ENTITY);
}
