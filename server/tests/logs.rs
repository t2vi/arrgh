//! S1 parity check for `/api/logs`. Mirrors `api-tests/tests/logs.hurl`.

use axum::body::Body;
use axum::http::{Request, StatusCode};
use http_body_util::BodyExt;
use jsonwebtoken::{encode, EncodingKey, Header};
use serde::Serialize;
use tower::ServiceExt; // oneshot

use arrgh_server::config::Config;
use arrgh_server::logs::LogBuffer;
use arrgh_server::state::AppState;

const SECRET: &str = "test-jwt-secret-for-logs-integration!!";

fn test_state() -> AppState {
    let config = Config {
        jwt_secret: Some(SECRET.into()),
        ..Config::from_env().expect("default config")
    };
    AppState::new(config, LogBuffer::new("info"))
}

#[derive(Serialize)]
struct TestClaims<'a> {
    #[serde(rename = "http://schemas.microsoft.com/ws/2008/06/identity/claims/role")]
    role: &'a str,
    exp: usize,
}

fn token(role: &str) -> String {
    let claims = TestClaims {
        role,
        exp: 9_999_999_999,
    };
    encode(
        &Header::default(),
        &claims,
        &EncodingKey::from_secret(SECRET.as_bytes()),
    )
    .unwrap()
}

#[tokio::test]
async fn list_requires_auth() {
    let app = arrgh_server::api::router(test_state());
    let res = app
        .oneshot(Request::get("/api/logs").body(Body::empty()).unwrap())
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn list_returns_array_with_valid_token() {
    let app = arrgh_server::api::router(test_state());
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
    let app = arrgh_server::api::router(test_state());
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
    let app = arrgh_server::api::router(test_state());

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
    let app = arrgh_server::api::router(test_state());
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
