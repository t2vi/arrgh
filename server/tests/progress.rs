//! S4 parity check for `/api/progress`. Mirrors `server-tests/ProgressTests.cs`
//! (ADR 0033, S4 #126).

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

// ── GET /api/progress/title/{titleId} ─────────────────────────────────────

#[tokio::test]
async fn list_title_progress_returns_progress_for_user() {
    let state = common::build_state().await;
    let admin = common::seed_user(&state, "admin", "admin", true).await;
    let t = common::seed_title(&state, "Naruto", false).await;
    common::seed_user_title(&state, &admin.id, &t).await;
    let c1 = common::seed_chapter(&state, &t, 1.0, false).await;
    common::seed_chapter(&state, &t, 2.0, false).await;
    common::mark_read(&state, &admin.id, &c1).await;
    let token = common::token_for(&admin);
    let app = arrgh_server::api::router(state);

    let (_, body) = send(
        &app,
        "GET",
        &format!("/api/progress/title/{t}"),
        Some(&token),
        None,
    )
    .await;
    assert_eq!(body.as_array().unwrap().len(), 1);
    assert_eq!(body[0]["chapter_id"], c1);
}

#[tokio::test]
async fn list_title_progress_isolated_per_user() {
    let state = common::build_state().await;
    let user1 = common::seed_user(&state, "admin", "admin", true).await;
    let user2 = common::seed_user(&state, "member1", "member", false).await;
    let t = common::seed_title(&state, "Naruto", false).await;
    common::seed_user_title(&state, &user1.id, &t).await;
    let c = common::seed_chapter(&state, &t, 1.0, false).await;
    common::mark_read(&state, &user2.id, &c).await; // user2 read it, not user1
    let token = common::token_for(&user1);
    let app = arrgh_server::api::router(state);

    let (_, body) = send(
        &app,
        "GET",
        &format!("/api/progress/title/{t}"),
        Some(&token),
        None,
    )
    .await;
    assert_eq!(body.as_array().unwrap().len(), 0);
}

// ── GET /api/progress/{chapterId} ─────────────────────────────────────────

#[tokio::test]
async fn get_progress_returns_progress() {
    let state = common::build_state().await;
    let admin = common::seed_user(&state, "admin", "admin", true).await;
    let t = common::seed_title(&state, "Naruto", false).await;
    common::seed_user_title(&state, &admin.id, &t).await;
    let c = common::seed_chapter(&state, &t, 1.0, false).await;
    common::mark_read(&state, &admin.id, &c).await;
    let token = common::token_for(&admin);
    let app = arrgh_server::api::router(state);

    let (status, body) = send(
        &app,
        "GET",
        &format!("/api/progress/{c}"),
        Some(&token),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["chapter_id"], c);
    assert_eq!(body["completed"], true);
}

#[tokio::test]
async fn get_progress_not_found_when_no_progress() {
    let state = common::build_state().await;
    let admin = common::seed_user(&state, "admin", "admin", true).await;
    let t = common::seed_title(&state, "Naruto", false).await;
    common::seed_user_title(&state, &admin.id, &t).await;
    let c = common::seed_chapter(&state, &t, 1.0, false).await;
    let token = common::token_for(&admin);
    let app = arrgh_server::api::router(state);

    let (status, _) = send(
        &app,
        "GET",
        &format!("/api/progress/{c}"),
        Some(&token),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn get_progress_unauthorized_no_token() {
    let app = arrgh_server::api::router(common::build_state().await);
    let (status, _) = send(&app, "GET", "/api/progress/any", None, None).await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
}

// ── PUT /api/progress/{chapterId} ─────────────────────────────────────────

#[tokio::test]
async fn update_progress_creates_when_not_exists() {
    let state = common::build_state().await;
    let admin = common::seed_user(&state, "admin", "admin", true).await;
    let t = common::seed_title(&state, "Naruto", false).await;
    common::seed_user_title(&state, &admin.id, &t).await;
    let c = common::seed_chapter(&state, &t, 1.0, false).await;
    let token = common::token_for(&admin);
    let app = arrgh_server::api::router(state);

    let (status, body) = send(
        &app,
        "PUT",
        &format!("/api/progress/{c}"),
        Some(&token),
        Some(json!({ "current_page": 5, "completed": false })),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["current_page"], 5);
    assert_eq!(body["completed"], false);
    assert_eq!(body["chapter_id"], c);
}

#[tokio::test]
async fn update_progress_updates_when_already_exists() {
    let state = common::build_state().await;
    let admin = common::seed_user(&state, "admin", "admin", true).await;
    let t = common::seed_title(&state, "Naruto", false).await;
    common::seed_user_title(&state, &admin.id, &t).await;
    let c = common::seed_chapter(&state, &t, 1.0, false).await;
    let token = common::token_for(&admin);
    let app = arrgh_server::api::router(state);

    send(
        &app,
        "PUT",
        &format!("/api/progress/{c}"),
        Some(&token),
        Some(json!({ "current_page": 3, "completed": false })),
    )
    .await;
    let (_, body) = send(
        &app,
        "PUT",
        &format!("/api/progress/{c}"),
        Some(&token),
        Some(json!({ "current_page": 10, "completed": true })),
    )
    .await;

    assert_eq!(body["current_page"], 10);
    assert_eq!(body["completed"], true);
}

#[tokio::test]
async fn update_progress_isolated_per_user() {
    let state = common::build_state().await;
    let user1 = common::seed_user(&state, "admin", "admin", true).await;
    let user2 = common::seed_user(&state, "member1", "member", false).await;
    let t = common::seed_title(&state, "Naruto", false).await;
    common::seed_user_title(&state, &user1.id, &t).await;
    let c = common::seed_chapter(&state, &t, 1.0, false).await;
    let app = arrgh_server::api::router(state);

    let token1 = common::token_for(&user1);
    send(
        &app,
        "PUT",
        &format!("/api/progress/{c}"),
        Some(&token1),
        Some(json!({ "current_page": 5, "completed": false })),
    )
    .await;

    let token2 = common::token_for(&user2);
    send(
        &app,
        "PUT",
        &format!("/api/progress/{c}"),
        Some(&token2),
        Some(json!({ "current_page": 99, "completed": true })),
    )
    .await;

    let (_, body) = send(
        &app,
        "GET",
        &format!("/api/progress/{c}"),
        Some(&token1),
        None,
    )
    .await;
    assert_eq!(body["current_page"], 5);
    assert_eq!(body["completed"], false);
}

// ── GET /api/progress/continue ────────────────────────────────────────────

#[tokio::test]
async fn continue_reading_returns_titles_with_unread_chapters() {
    let state = common::build_state().await;
    let admin = common::seed_user(&state, "admin", "admin", true).await;
    let t = common::seed_title(&state, "Naruto", false).await;
    common::seed_user_title(&state, &admin.id, &t).await;
    let c1 = common::seed_chapter(&state, &t, 1.0, true).await;
    common::seed_chapter(&state, &t, 2.0, true).await;
    common::mark_read(&state, &admin.id, &c1).await;
    let token = common::token_for(&admin);
    let app = arrgh_server::api::router(state);

    let (_, body) = send(&app, "GET", "/api/progress/continue", Some(&token), None).await;
    assert_eq!(body.as_array().unwrap().len(), 1);
    assert_eq!(body[0]["title_id"], t);
    assert_eq!(body[0]["chapter_number"], 2.0);
    assert_eq!(body[0]["chapters_read"], 1);
    assert_eq!(body[0]["total_chapters"], 2);
}

#[tokio::test]
async fn continue_reading_empty_when_nothing_started() {
    let state = common::build_state().await;
    let admin = common::seed_user(&state, "admin", "admin", true).await;
    let t = common::seed_title(&state, "Naruto", false).await;
    common::seed_user_title(&state, &admin.id, &t).await;
    common::seed_chapter(&state, &t, 1.0, true).await;
    let token = common::token_for(&admin);
    let app = arrgh_server::api::router(state);

    let (_, body) = send(&app, "GET", "/api/progress/continue", Some(&token), None).await;
    assert_eq!(body.as_array().unwrap().len(), 0);
}

#[tokio::test]
async fn continue_reading_empty_when_all_chapters_read() {
    let state = common::build_state().await;
    let admin = common::seed_user(&state, "admin", "admin", true).await;
    let t = common::seed_title(&state, "Naruto", false).await;
    common::seed_user_title(&state, &admin.id, &t).await;
    let c = common::seed_chapter(&state, &t, 1.0, true).await;
    common::mark_read(&state, &admin.id, &c).await;
    let token = common::token_for(&admin);
    let app = arrgh_server::api::router(state);

    let (_, body) = send(&app, "GET", "/api/progress/continue", Some(&token), None).await;
    assert_eq!(body.as_array().unwrap().len(), 0);
}

#[tokio::test]
async fn continue_reading_skips_not_downloaded_chapters() {
    let state = common::build_state().await;
    let admin = common::seed_user(&state, "admin", "admin", true).await;
    let t = common::seed_title(&state, "Naruto", false).await;
    common::seed_user_title(&state, &admin.id, &t).await;
    let c1 = common::seed_chapter(&state, &t, 1.0, true).await;
    common::seed_chapter(&state, &t, 2.0, false).await; // not downloaded
    common::seed_chapter(&state, &t, 3.0, true).await;
    common::mark_read(&state, &admin.id, &c1).await;
    let token = common::token_for(&admin);
    let app = arrgh_server::api::router(state);

    let (_, body) = send(&app, "GET", "/api/progress/continue", Some(&token), None).await;
    assert_eq!(body.as_array().unwrap().len(), 1);
    assert_eq!(body[0]["chapter_number"], 3.0);
}
