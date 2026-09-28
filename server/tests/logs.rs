//! S1 parity check for `/api/logs`. Mirrors `api-tests/tests/logs.hurl`.

mod common;

use axum::body::Body;
use axum::http::{Request, StatusCode};
use http_body_util::BodyExt;
use tower::ServiceExt; // oneshot

fn token(role: &str) -> String {
    arrgh_server::auth::create_token("user-1", "tester", role, false, common::JWT_SECRET).unwrap()
}

// spec: 008/FR-008
#[tokio::test]
async fn list_requires_auth() {
    let app = arrgh_server::api::router(common::build_state().await);
    let res = app
        .oneshot(Request::get("/api/logs").body(Body::empty()).unwrap())
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::UNAUTHORIZED);
}

// spec: 008/FR-008
#[tokio::test]
async fn get_level_requires_auth() {
    let app = arrgh_server::api::router(common::build_state().await);
    let res = app
        .oneshot(Request::get("/api/logs/level").body(Body::empty()).unwrap())
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::UNAUTHORIZED);
}

// spec: 008/FR-002
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

// spec: 008/FR-003
#[tokio::test]
async fn list_respects_limit_query_param() {
    let state = common::build_state().await;
    for i in 0..5 {
        state.logs.append_for_test(arrgh_server::logs::LogEntry {
            timestamp: "t".into(),
            level: "INFO".into(),
            target: "arrgh_server".into(),
            message: i.to_string(),
        });
    }
    let app = arrgh_server::api::router(state);

    let res = app
        .oneshot(
            Request::get("/api/logs?limit=2")
                .header("authorization", format!("Bearer {}", token("member")))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let bytes = res.into_body().collect().await.unwrap().to_bytes();
    let body: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    let arr = body.as_array().unwrap();
    assert_eq!(arr.len(), 2);
    assert_eq!(arr[0]["message"], "3");
    assert_eq!(arr[1]["message"], "4");
}

// spec: 008/FR-007
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

// spec: 008/FR-005, 008/FR-006
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

// spec: 008/FR-007
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
