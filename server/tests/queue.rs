//! S7 parity check for `/api/queue`. Mirrors `server-tests/QueueTests.cs`
//! (ADR 0033, S7 #129).

mod common;

use axum::body::Body;
use axum::http::{Request, StatusCode};
use axum::Router;
use http_body_util::BodyExt;
use serde_json::Value;
use tower::ServiceExt; // oneshot

async fn send(app: &Router, method: &str, uri: &str, token: Option<&str>) -> (StatusCode, Value) {
    let mut builder = Request::builder().method(method).uri(uri);
    if let Some(t) = token {
        builder = builder.header("authorization", format!("Bearer {t}"));
    }
    let req = builder.body(Body::empty()).unwrap();

    let res = app.clone().oneshot(req).await.unwrap();
    let status = res.status();
    let bytes = res.into_body().collect().await.unwrap().to_bytes();
    let json = serde_json::from_slice(&bytes).unwrap_or(Value::Null);
    (status, json)
}

// ── GET /api/queue ───────────────────────────────────────────────────────

#[tokio::test]
async fn list_empty_when_no_items() {
    let state = common::build_state().await;
    let admin = common::seed_user(&state, "admin", "admin", true).await;
    let token = common::token_for(&admin);
    let app = arrgh_server::api::router(state);

    let (status, body) = send(&app, "GET", "/api/queue", Some(&token)).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body.as_array().unwrap().len(), 0);
}

#[tokio::test]
async fn list_ordered_by_created_at_desc() {
    let state = common::build_state().await;
    let admin = common::seed_user(&state, "admin", "admin", true).await;
    let t = common::seed_title(&state, "Naruto", false).await;
    let c1 = common::seed_chapter(&state, &t, 1.0, false).await;
    let c2 = common::seed_chapter(&state, &t, 2.0, false).await;
    common::seed_queue_item(&state, &c1, "Naruto", 1.0, "pending", Some(&admin.id)).await;
    common::seed_queue_item(&state, &c2, "Naruto", 2.0, "pending", Some(&admin.id)).await;
    let token = common::token_for(&admin);
    let app = arrgh_server::api::router(state);

    let (_, body) = send(&app, "GET", "/api/queue", Some(&token)).await;
    let arr = body.as_array().unwrap();
    assert_eq!(arr.len(), 2);
    // most-recently-created first
    assert_eq!(arr[0]["chapter_num"], 2.0);
    assert_eq!(arr[1]["chapter_num"], 1.0);
}

#[tokio::test]
async fn list_hides_explicit_title_for_member_without_flag() {
    let state = common::build_state().await;
    let member = common::seed_user(&state, "member1", "member", false).await;
    let t = common::seed_title(&state, "Hentai", true).await;
    let c = common::seed_chapter(&state, &t, 1.0, false).await;
    common::seed_queue_item(&state, &c, "Hentai", 1.0, "pending", Some(&member.id)).await;
    let token = common::token_for(&member);
    let app = arrgh_server::api::router(state);

    let (_, body) = send(&app, "GET", "/api/queue", Some(&token)).await;
    assert_eq!(body.as_array().unwrap().len(), 0);
}

#[tokio::test]
async fn list_shows_explicit_title_for_admin_regardless_of_flag() {
    let state = common::build_state().await;
    let admin = common::seed_user(&state, "admin", "admin", false).await;
    let t = common::seed_title(&state, "Hentai", true).await;
    let c = common::seed_chapter(&state, &t, 1.0, false).await;
    common::seed_queue_item(&state, &c, "Hentai", 1.0, "pending", Some(&admin.id)).await;
    let token = common::token_for(&admin);
    let app = arrgh_server::api::router(state);

    let (_, body) = send(&app, "GET", "/api/queue", Some(&token)).await;
    assert_eq!(body.as_array().unwrap().len(), 1);
}

#[tokio::test]
async fn list_unauthorized_no_token() {
    let app = arrgh_server::api::router(common::build_state().await);
    let (status, _) = send(&app, "GET", "/api/queue", None).await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
}

// ── GET /api/queue/title/{titleId} ─────────────────────────────────────────

#[tokio::test]
async fn list_for_title_filtered_and_ordered_by_chapter_num() {
    let state = common::build_state().await;
    let admin = common::seed_user(&state, "admin", "admin", true).await;
    let t1 = common::seed_title(&state, "Naruto", false).await;
    let t2 = common::seed_title(&state, "Bleach", false).await;
    let c1 = common::seed_chapter(&state, &t1, 3.0, false).await;
    let c2 = common::seed_chapter(&state, &t1, 1.0, false).await;
    let c3 = common::seed_chapter(&state, &t2, 1.0, false).await;
    common::seed_queue_item(&state, &c1, "Naruto", 3.0, "pending", Some(&admin.id)).await;
    common::seed_queue_item(&state, &c2, "Naruto", 1.0, "pending", Some(&admin.id)).await;
    common::seed_queue_item(&state, &c3, "Bleach", 1.0, "pending", Some(&admin.id)).await;
    let token = common::token_for(&admin);
    let app = arrgh_server::api::router(state);

    let (_, body) = send(&app, "GET", &format!("/api/queue/title/{t1}"), Some(&token)).await;
    let arr = body.as_array().unwrap();
    assert_eq!(arr.len(), 2);
    assert_eq!(arr[0]["chapter_num"], 1.0);
    assert_eq!(arr[1]["chapter_num"], 3.0);
}

#[tokio::test]
async fn list_for_title_empty_when_none() {
    let state = common::build_state().await;
    let admin = common::seed_user(&state, "admin", "admin", true).await;
    let t = common::seed_title(&state, "Naruto", false).await;
    let token = common::token_for(&admin);
    let app = arrgh_server::api::router(state);

    let (_, body) = send(&app, "GET", &format!("/api/queue/title/{t}"), Some(&token)).await;
    assert_eq!(body.as_array().unwrap().len(), 0);
}

// ── DELETE /api/queue/completed ─────────────────────────────────────────────

#[tokio::test]
async fn clear_completed_deletes_done_cancelled_error_leaves_pending() {
    let state = common::build_state().await;
    let admin = common::seed_user(&state, "admin", "admin", true).await;
    let t = common::seed_title(&state, "Naruto", false).await;
    let c1 = common::seed_chapter(&state, &t, 1.0, false).await;
    let c2 = common::seed_chapter(&state, &t, 2.0, false).await;
    let c3 = common::seed_chapter(&state, &t, 3.0, false).await;
    let c4 = common::seed_chapter(&state, &t, 4.0, false).await;
    common::seed_queue_item(&state, &c1, "Naruto", 1.0, "done", None).await;
    common::seed_queue_item(&state, &c2, "Naruto", 2.0, "cancelled", None).await;
    common::seed_queue_item(&state, &c3, "Naruto", 3.0, "error", None).await;
    common::seed_queue_item(&state, &c4, "Naruto", 4.0, "pending", None).await;
    let token = common::token_for(&admin);
    let app = arrgh_server::api::router(state.clone());

    let (status, _) = send(&app, "DELETE", "/api/queue/completed", Some(&token)).await;
    assert_eq!(status, StatusCode::NO_CONTENT);

    let remaining: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM download_queue")
        .fetch_one(&state.db)
        .await
        .unwrap();
    assert_eq!(remaining, 1);
    let remaining_status: String = sqlx::query_scalar("SELECT status FROM download_queue LIMIT 1")
        .fetch_one(&state.db)
        .await
        .unwrap();
    assert_eq!(remaining_status, "pending");
}

#[tokio::test]
async fn clear_completed_forbidden_for_member() {
    let state = common::build_state().await;
    let member = common::seed_user(&state, "member1", "member", false).await;
    let token = common::token_for(&member);
    let app = arrgh_server::api::router(state);

    let (status, _) = send(&app, "DELETE", "/api/queue/completed", Some(&token)).await;
    assert_eq!(status, StatusCode::FORBIDDEN);
}

// ── DELETE /api/queue/{id} ───────────────────────────────────────────────────

#[tokio::test]
async fn remove_deletes_when_not_downloading() {
    let state = common::build_state().await;
    let admin = common::seed_user(&state, "admin", "admin", true).await;
    let t = common::seed_title(&state, "Naruto", false).await;
    let c = common::seed_chapter(&state, &t, 1.0, false).await;
    let id = common::seed_queue_item(&state, &c, "Naruto", 1.0, "pending", Some(&admin.id)).await;
    let token = common::token_for(&admin);
    let app = arrgh_server::api::router(state.clone());

    let (status, _) = send(&app, "DELETE", &format!("/api/queue/{id}"), Some(&token)).await;
    assert_eq!(status, StatusCode::NO_CONTENT);

    let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM download_queue WHERE id = ?")
        .bind(&id)
        .fetch_one(&state.db)
        .await
        .unwrap();
    assert_eq!(count, 0);
}

#[tokio::test]
async fn remove_soft_cancels_when_downloading() {
    let state = common::build_state().await;
    let admin = common::seed_user(&state, "admin", "admin", true).await;
    let t = common::seed_title(&state, "Naruto", false).await;
    let c = common::seed_chapter(&state, &t, 1.0, false).await;
    let id =
        common::seed_queue_item(&state, &c, "Naruto", 1.0, "downloading", Some(&admin.id)).await;
    let token = common::token_for(&admin);
    let app = arrgh_server::api::router(state.clone());

    let (status, _) = send(&app, "DELETE", &format!("/api/queue/{id}"), Some(&token)).await;
    assert_eq!(status, StatusCode::NO_CONTENT);

    let row_status: String = sqlx::query_scalar("SELECT status FROM download_queue WHERE id = ?")
        .bind(&id)
        .fetch_one(&state.db)
        .await
        .unwrap();
    assert_eq!(row_status, "cancelled");
}

#[tokio::test]
async fn remove_not_found_nonexistent() {
    let state = common::build_state().await;
    let admin = common::seed_user(&state, "admin", "admin", true).await;
    let token = common::token_for(&admin);
    let app = arrgh_server::api::router(state);

    let (status, _) = send(&app, "DELETE", "/api/queue/ghost", Some(&token)).await;
    assert_eq!(status, StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn remove_forbidden_for_other_users_item() {
    let state = common::build_state().await;
    let owner = common::seed_user(&state, "owner", "member", false).await;
    let other = common::seed_user(&state, "other", "member", false).await;
    let t = common::seed_title(&state, "Naruto", false).await;
    let c = common::seed_chapter(&state, &t, 1.0, false).await;
    let id = common::seed_queue_item(&state, &c, "Naruto", 1.0, "pending", Some(&owner.id)).await;
    let token = common::token_for(&other);
    let app = arrgh_server::api::router(state);

    let (status, _) = send(&app, "DELETE", &format!("/api/queue/{id}"), Some(&token)).await;
    assert_eq!(status, StatusCode::FORBIDDEN);
}

#[tokio::test]
async fn remove_allowed_for_admin_on_others_item() {
    let state = common::build_state().await;
    let owner = common::seed_user(&state, "owner", "member", false).await;
    let admin = common::seed_user(&state, "admin", "admin", true).await;
    let t = common::seed_title(&state, "Naruto", false).await;
    let c = common::seed_chapter(&state, &t, 1.0, false).await;
    let id = common::seed_queue_item(&state, &c, "Naruto", 1.0, "pending", Some(&owner.id)).await;
    let token = common::token_for(&admin);
    let app = arrgh_server::api::router(state.clone());

    let (status, _) = send(&app, "DELETE", &format!("/api/queue/{id}"), Some(&token)).await;
    assert_eq!(status, StatusCode::NO_CONTENT);

    let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM download_queue WHERE id = ?")
        .bind(&id)
        .fetch_one(&state.db)
        .await
        .unwrap();
    assert_eq!(count, 0);
}

#[tokio::test]
async fn remove_allowed_for_owning_member() {
    let state = common::build_state().await;
    let owner = common::seed_user(&state, "owner", "member", false).await;
    let t = common::seed_title(&state, "Naruto", false).await;
    let c = common::seed_chapter(&state, &t, 1.0, false).await;
    let id = common::seed_queue_item(&state, &c, "Naruto", 1.0, "pending", Some(&owner.id)).await;
    let token = common::token_for(&owner);
    let app = arrgh_server::api::router(state);

    let (status, _) = send(&app, "DELETE", &format!("/api/queue/{id}"), Some(&token)).await;
    assert_eq!(status, StatusCode::NO_CONTENT);
}
