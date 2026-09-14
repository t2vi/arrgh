//! S5 parity check for `/api/chapters`. Mirrors `server-tests/ChaptersTests.cs`
//! (ADR 0033, S5 #127). `Api/ChapterSync.cs`'s parity checks live in
//! `tests/chapter_sync.rs`.

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

// ── GET /api/chapters/title/{titleId} ─────────────────────────────────────

#[tokio::test]
async fn list_chapters_returns_chapters_ordered_by_number() {
    let state = common::build_state().await;
    let admin = common::seed_user(&state, "admin", "admin", true).await;
    let t = common::seed_title(&state, "Naruto", false).await;
    common::seed_user_title(&state, &admin.id, &t).await;
    common::seed_chapter(&state, &t, 3.0, false).await;
    common::seed_chapter(&state, &t, 1.0, false).await;
    common::seed_chapter(&state, &t, 2.0, false).await;
    let token = common::token_for(&admin);
    let app = arrgh_server::api::router(state);

    let (_, body) = send(
        &app,
        "GET",
        &format!("/api/chapters/title/{t}"),
        Some(&token),
    )
    .await;
    let arr = body.as_array().unwrap();
    assert_eq!(arr.len(), 3);
    assert_eq!(arr[0]["number"], 1.0);
    assert_eq!(arr[1]["number"], 2.0);
    assert_eq!(arr[2]["number"], 3.0);
}

#[tokio::test]
async fn list_chapters_returns_empty_no_chapters() {
    let state = common::build_state().await;
    let admin = common::seed_user(&state, "admin", "admin", true).await;
    let t = common::seed_title(&state, "Naruto", false).await;
    common::seed_user_title(&state, &admin.id, &t).await;
    let token = common::token_for(&admin);
    let app = arrgh_server::api::router(state);

    let (_, body) = send(
        &app,
        "GET",
        &format!("/api/chapters/title/{t}"),
        Some(&token),
    )
    .await;
    assert_eq!(body.as_array().unwrap().len(), 0);
}

#[tokio::test]
async fn list_chapters_has_sources_true_when_chapter_source_exists() {
    let state = common::build_state().await;
    let admin = common::seed_user(&state, "admin", "admin", true).await;
    let t = common::seed_title(&state, "Naruto", false).await;
    common::seed_user_title(&state, &admin.id, &t).await;
    let c = common::seed_chapter(&state, &t, 1.0, false).await;
    common::add_chapter_source(&state, &c, "mangadex").await;
    let token = common::token_for(&admin);
    let app = arrgh_server::api::router(state);

    let (_, body) = send(
        &app,
        "GET",
        &format!("/api/chapters/title/{t}"),
        Some(&token),
    )
    .await;
    assert_eq!(body[0]["has_sources"], true);
}

#[tokio::test]
async fn list_chapters_has_sources_false_when_no_chapter_source() {
    let state = common::build_state().await;
    let admin = common::seed_user(&state, "admin", "admin", true).await;
    let t = common::seed_title(&state, "Naruto", false).await;
    common::seed_user_title(&state, &admin.id, &t).await;
    common::seed_chapter(&state, &t, 1.0, false).await;
    let token = common::token_for(&admin);
    let app = arrgh_server::api::router(state);

    let (_, body) = send(
        &app,
        "GET",
        &format!("/api/chapters/title/{t}"),
        Some(&token),
    )
    .await;
    assert_eq!(body[0]["has_sources"], false);
}

#[tokio::test]
async fn list_chapters_hides_explicit_title_when_user_not_allowed() {
    let state = common::build_state().await;
    let member = common::seed_user(&state, "member1", "member", false).await;
    let t = common::seed_title(&state, "Hentai", true).await;
    common::seed_user_title(&state, &member.id, &t).await;
    common::seed_chapter(&state, &t, 1.0, false).await;
    let token = common::token_for(&member);
    let app = arrgh_server::api::router(state);

    let (_, body) = send(
        &app,
        "GET",
        &format!("/api/chapters/title/{t}"),
        Some(&token),
    )
    .await;
    assert_eq!(body.as_array().unwrap().len(), 0);
}

#[tokio::test]
async fn list_chapters_shows_explicit_title_when_user_allowed() {
    let state = common::build_state().await;
    let admin = common::seed_user(&state, "admin", "admin", true).await;
    let t = common::seed_title(&state, "Hentai", true).await;
    common::seed_user_title(&state, &admin.id, &t).await;
    common::seed_chapter(&state, &t, 1.0, false).await;
    let token = common::token_for(&admin);
    let app = arrgh_server::api::router(state);

    let (_, body) = send(
        &app,
        "GET",
        &format!("/api/chapters/title/{t}"),
        Some(&token),
    )
    .await;
    assert_eq!(body.as_array().unwrap().len(), 1);
}

#[tokio::test]
async fn list_chapters_unauthorized_no_token() {
    let app = arrgh_server::api::router(common::build_state().await);
    let (status, _) = send(&app, "GET", "/api/chapters/title/any", None).await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
}

// ── GET /api/chapters/{id} ─────────────────────────────────────────────────

#[tokio::test]
async fn get_chapter_returns_chapter() {
    let state = common::build_state().await;
    let admin = common::seed_user(&state, "admin", "admin", true).await;
    let t = common::seed_title(&state, "Naruto", false).await;
    common::seed_user_title(&state, &admin.id, &t).await;
    let c = common::seed_chapter(&state, &t, 5.0, false).await;
    let token = common::token_for(&admin);
    let app = arrgh_server::api::router(state);

    let (status, body) = send(&app, "GET", &format!("/api/chapters/{c}"), Some(&token)).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["id"], c);
    assert_eq!(body["number"], 5.0);
}

#[tokio::test]
async fn get_chapter_not_found_nonexistent() {
    let state = common::build_state().await;
    let admin = common::seed_user(&state, "admin", "admin", true).await;
    let token = common::token_for(&admin);
    let app = arrgh_server::api::router(state);

    let (status, _) = send(&app, "GET", "/api/chapters/ghost", Some(&token)).await;
    assert_eq!(status, StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn get_chapter_not_found_explicit_hidden_from_user() {
    let state = common::build_state().await;
    let member = common::seed_user(&state, "member1", "member", false).await;
    let t = common::seed_title(&state, "Hentai", true).await;
    common::seed_user_title(&state, &member.id, &t).await;
    let c = common::seed_chapter(&state, &t, 1.0, false).await;
    let token = common::token_for(&member);
    let app = arrgh_server::api::router(state);

    let (status, _) = send(&app, "GET", &format!("/api/chapters/{c}"), Some(&token)).await;
    assert_eq!(status, StatusCode::NOT_FOUND);
}

// ── GET /api/chapters/{id}/text ────────────────────────────────────────────

#[tokio::test]
async fn get_chapter_text_bad_request_when_not_text_format() {
    let state = common::build_state().await;
    let admin = common::seed_user(&state, "admin", "admin", true).await;
    let t = common::seed_title(&state, "Naruto", false).await;
    common::seed_user_title(&state, &admin.id, &t).await;
    let c = common::seed_chapter(&state, &t, 1.0, false).await; // chapter_format = "cbz"
    let token = common::token_for(&admin);
    let app = arrgh_server::api::router(state);

    let (status, _) = send(
        &app,
        "GET",
        &format!("/api/chapters/{c}/text"),
        Some(&token),
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn get_chapter_text_not_found_when_not_downloaded() {
    let state = common::build_state().await;
    let admin = common::seed_user(&state, "admin", "admin", true).await;
    let t = common::seed_title(&state, "Naruto", false).await;
    common::seed_user_title(&state, &admin.id, &t).await;
    let c = common::seed_chapter(&state, &t, 1.0, false).await;
    sqlx::query("UPDATE chapters SET chapter_format = 'text' WHERE id = ?")
        .bind(&c)
        .execute(&state.db)
        .await
        .unwrap();
    let token = common::token_for(&admin);
    let app = arrgh_server::api::router(state);

    let (status, _) = send(
        &app,
        "GET",
        &format!("/api/chapters/{c}/text"),
        Some(&token),
    )
    .await;
    assert_eq!(status, StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn get_chapter_text_not_found_when_file_gone() {
    let state = common::build_state().await;
    let admin = common::seed_user(&state, "admin", "admin", true).await;
    let t = common::seed_title(&state, "Naruto", false).await;
    common::seed_user_title(&state, &admin.id, &t).await;
    let c = common::seed_chapter(&state, &t, 1.0, true).await;
    sqlx::query("UPDATE chapters SET chapter_format = 'text', local_path = '/nonexistent/path/novel.txt' WHERE id = ?")
        .bind(&c)
        .execute(&state.db)
        .await
        .unwrap();
    let token = common::token_for(&admin);
    let app = arrgh_server::api::router(state);

    let (status, _) = send(
        &app,
        "GET",
        &format!("/api/chapters/{c}/text"),
        Some(&token),
    )
    .await;
    assert_eq!(status, StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn get_chapter_text_returns_content_when_file_exists() {
    let state = common::build_state().await;
    let admin = common::seed_user(&state, "admin", "admin", true).await;
    let t = common::seed_title(&state, "Naruto", false).await;
    common::seed_user_title(&state, &admin.id, &t).await;
    let c = common::seed_chapter(&state, &t, 1.0, true).await;

    let tmp = std::env::temp_dir().join(format!("arrgh-chaptext-{}.txt", uuid::Uuid::new_v4()));
    std::fs::write(&tmp, "Chapter one content.").unwrap();
    sqlx::query("UPDATE chapters SET chapter_format = 'text', local_path = ? WHERE id = ?")
        .bind(tmp.to_str().unwrap())
        .bind(&c)
        .execute(&state.db)
        .await
        .unwrap();
    let token = common::token_for(&admin);
    let app = arrgh_server::api::router(state);

    let (status, body) = send(
        &app,
        "GET",
        &format!("/api/chapters/{c}/text"),
        Some(&token),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["content"], "Chapter one content.");

    std::fs::remove_file(&tmp).unwrap();
}

// ── POST /api/chapters/{id}/download ───────────────────────────────────────

#[tokio::test]
async fn queue_download_accepted_when_eligible() {
    let state = common::build_state().await;
    let admin = common::seed_user(&state, "admin", "admin", true).await;
    let t = common::seed_title(&state, "Naruto", false).await;
    common::seed_user_title(&state, &admin.id, &t).await;
    let c = common::seed_chapter(&state, &t, 1.0, false).await;
    common::add_chapter_source(&state, &c, "mangadex").await;
    let token = common::token_for(&admin);
    let app = arrgh_server::api::router(state.clone());

    let (status, _) = send(
        &app,
        "POST",
        &format!("/api/chapters/{c}/download"),
        Some(&token),
    )
    .await;
    assert_eq!(status, StatusCode::ACCEPTED);

    let (status_val, queued_by): (String, Option<String>) =
        sqlx::query_as("SELECT status, queued_by FROM download_queue WHERE chapter_id = ?")
            .bind(&c)
            .fetch_one(&state.db)
            .await
            .unwrap();
    assert_eq!(status_val, "pending");
    assert_eq!(queued_by.as_deref(), Some(admin.id.as_str()));
}

#[tokio::test]
async fn queue_download_not_found_when_already_downloaded() {
    let state = common::build_state().await;
    let admin = common::seed_user(&state, "admin", "admin", true).await;
    let t = common::seed_title(&state, "Naruto", false).await;
    common::seed_user_title(&state, &admin.id, &t).await;
    let c = common::seed_chapter(&state, &t, 1.0, true).await; // already downloaded
    common::add_chapter_source(&state, &c, "mangadex").await;
    let token = common::token_for(&admin);
    let app = arrgh_server::api::router(state);

    let (status, _) = send(
        &app,
        "POST",
        &format!("/api/chapters/{c}/download"),
        Some(&token),
    )
    .await;
    assert_eq!(status, StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn queue_download_not_found_when_no_chapter_source() {
    let state = common::build_state().await;
    let admin = common::seed_user(&state, "admin", "admin", true).await;
    let t = common::seed_title(&state, "Naruto", false).await;
    common::seed_user_title(&state, &admin.id, &t).await;
    let c = common::seed_chapter(&state, &t, 1.0, false).await; // no chapter_sources
    let token = common::token_for(&admin);
    let app = arrgh_server::api::router(state);

    let (status, _) = send(
        &app,
        "POST",
        &format!("/api/chapters/{c}/download"),
        Some(&token),
    )
    .await;
    assert_eq!(status, StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn queue_download_not_found_explicit_hidden_from_user() {
    let state = common::build_state().await;
    let member = common::seed_user(&state, "member1", "member", false).await;
    let t = common::seed_title(&state, "Hentai", true).await;
    common::seed_user_title(&state, &member.id, &t).await;
    let c = common::seed_chapter(&state, &t, 1.0, false).await;
    common::add_chapter_source(&state, &c, "mangadex").await;
    let token = common::token_for(&member);
    let app = arrgh_server::api::router(state);

    let (status, _) = send(
        &app,
        "POST",
        &format!("/api/chapters/{c}/download"),
        Some(&token),
    )
    .await;
    assert_eq!(status, StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn queue_download_requeues_when_previously_errored() {
    let state = common::build_state().await;
    let admin = common::seed_user(&state, "admin", "admin", true).await;
    let t = common::seed_title(&state, "Naruto", false).await;
    common::seed_user_title(&state, &admin.id, &t).await;
    let c = common::seed_chapter(&state, &t, 1.0, false).await;
    common::add_chapter_source(&state, &c, "mangadex").await;
    common::seed_errored_queue_item(&state, &c, "Naruto", 1.0).await;
    let token = common::token_for(&admin);
    let app = arrgh_server::api::router(state.clone());

    let (status, _) = send(
        &app,
        "POST",
        &format!("/api/chapters/{c}/download"),
        Some(&token),
    )
    .await;
    assert_eq!(status, StatusCode::ACCEPTED);

    let (status_val, error): (String, Option<String>) =
        sqlx::query_as("SELECT status, error FROM download_queue WHERE chapter_id = ?")
            .bind(&c)
            .fetch_one(&state.db)
            .await
            .unwrap();
    assert_eq!(status_val, "pending");
    assert_eq!(error, None);
}

#[tokio::test]
async fn queue_download_unauthorized_no_token() {
    let app = arrgh_server::api::router(common::build_state().await);
    let (status, _) = send(&app, "POST", "/api/chapters/any/download", None).await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
}
