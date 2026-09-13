//! S4 parity check for `/api/titles`. Mirrors `server-tests/TitlesTests.cs`
//! (ADR 0033, S4 #126). Not every .NET case is ported — `RefreshMetadata`'s
//! MangaUpdates/Discover integration and the async sync-log timing tests are
//! covered at the level the Rust stub actually implements (see
//! `src/api/titles.rs`'s module doc).

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

// ── GET /api/titles ──────────────────────────────────────────────────────

#[tokio::test]
async fn list_titles_empty_library_returns_empty_page() {
    let state = common::build_state().await;
    let admin = common::seed_user(&state, "admin", "admin", true).await;
    let token = common::token_for(&admin);
    let app = arrgh_server::api::router(state);

    let (_, body) = send(&app, "GET", "/api/titles", Some(&token), None).await;
    assert_eq!(body["total"], 0);
    assert_eq!(body["items"].as_array().unwrap().len(), 0);
}

#[tokio::test]
async fn list_titles_returns_owned_titles() {
    let state = common::build_state().await;
    let admin = common::seed_user(&state, "admin", "admin", true).await;
    let t1 = common::seed_title(&state, "Naruto", false).await;
    let t2 = common::seed_title(&state, "Bleach", false).await;
    common::seed_user_title(&state, &admin.id, &t1).await;
    common::seed_user_title(&state, &admin.id, &t2).await;
    let token = common::token_for(&admin);
    let app = arrgh_server::api::router(state);

    let (_, body) = send(&app, "GET", "/api/titles", Some(&token), None).await;
    assert_eq!(body["total"], 2);
}

#[tokio::test]
async fn list_titles_excludes_other_users_library() {
    let state = common::build_state().await;
    let user1 = common::seed_user(&state, "admin", "admin", true).await;
    let user2 = common::seed_user(&state, "member1", "member", false).await;
    let t = common::seed_title(&state, "Solo", false).await;
    common::seed_user_title(&state, &user2.id, &t).await;
    let token = common::token_for(&user1);
    let app = arrgh_server::api::router(state);

    let (_, body) = send(&app, "GET", "/api/titles", Some(&token), None).await;
    assert_eq!(body["total"], 0);
}

#[tokio::test]
async fn list_titles_hides_explicit_from_non_explicit_user() {
    let state = common::build_state().await;
    let member = common::seed_user(&state, "member1", "member", false).await;
    let t = common::seed_title(&state, "Hentai Title", true).await;
    common::seed_user_title(&state, &member.id, &t).await;
    let token = common::token_for(&member);
    let app = arrgh_server::api::router(state);

    let (_, body) = send(&app, "GET", "/api/titles", Some(&token), None).await;
    assert_eq!(body["total"], 0);
}

#[tokio::test]
async fn list_titles_shows_explicit_to_explicit_user() {
    let state = common::build_state().await;
    let admin = common::seed_user(&state, "admin", "admin", true).await;
    let t = common::seed_title(&state, "Hentai Title", true).await;
    common::seed_user_title(&state, &admin.id, &t).await;
    let token = common::token_for(&admin);
    let app = arrgh_server::api::router(state);

    let (_, body) = send(&app, "GET", "/api/titles", Some(&token), None).await;
    assert_eq!(body["total"], 1);
}

#[tokio::test]
async fn list_titles_search_filters_by_title() {
    let state = common::build_state().await;
    let admin = common::seed_user(&state, "admin", "admin", true).await;
    let t1 = common::seed_title(&state, "Naruto", false).await;
    let t2 = common::seed_title(&state, "Bleach", false).await;
    common::seed_user_title(&state, &admin.id, &t1).await;
    common::seed_user_title(&state, &admin.id, &t2).await;
    let token = common::token_for(&admin);
    let app = arrgh_server::api::router(state);

    let (_, body) = send(&app, "GET", "/api/titles?search=Naruto", Some(&token), None).await;
    assert_eq!(body["total"], 1);
    assert_eq!(body["items"][0]["title"], "Naruto");
}

#[tokio::test]
async fn list_titles_unauthorized_no_token() {
    let app = arrgh_server::api::router(common::build_state().await);
    let (status, _) = send(&app, "GET", "/api/titles", None, None).await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn list_titles_pagination_limit_applied() {
    let state = common::build_state().await;
    let admin = common::seed_user(&state, "admin", "admin", true).await;
    for i in 0..5 {
        let t = common::seed_title(&state, &format!("Title {i}"), false).await;
        common::seed_user_title(&state, &admin.id, &t).await;
    }
    let token = common::token_for(&admin);
    let app = arrgh_server::api::router(state);

    let (_, body) = send(&app, "GET", "/api/titles?limit=2", Some(&token), None).await;
    assert_eq!(body["total"], 5);
    assert_eq!(body["items"].as_array().unwrap().len(), 2);
    assert_eq!(body["limit"], 2);
}

// ── GET /api/titles/{id} ─────────────────────────────────────────────────

#[tokio::test]
async fn get_title_returns_title_when_owned() {
    let state = common::build_state().await;
    let admin = common::seed_user(&state, "admin", "admin", true).await;
    let t = common::seed_title(&state, "Naruto", false).await;
    common::seed_user_title(&state, &admin.id, &t).await;
    let token = common::token_for(&admin);
    let app = arrgh_server::api::router(state);

    let (status, body) = send(&app, "GET", &format!("/api/titles/{t}"), Some(&token), None).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["id"], t);
    assert_eq!(body["title"], "Naruto");
}

#[tokio::test]
async fn get_title_not_found_when_not_owned() {
    let state = common::build_state().await;
    let admin = common::seed_user(&state, "admin", "admin", true).await;
    let t = common::seed_title(&state, "Naruto", false).await; // no user_titles row
    let token = common::token_for(&admin);
    let app = arrgh_server::api::router(state);

    let (status, _) = send(&app, "GET", &format!("/api/titles/{t}"), Some(&token), None).await;
    assert_eq!(status, StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn get_title_reports_chapter_stats() {
    let state = common::build_state().await;
    let admin = common::seed_user(&state, "admin", "admin", true).await;
    let t = common::seed_title(&state, "Naruto", false).await;
    common::seed_user_title(&state, &admin.id, &t).await;
    let c1 = common::seed_chapter(&state, &t, 1.0, true).await;
    common::seed_chapter(&state, &t, 2.0, true).await;
    common::seed_chapter(&state, &t, 3.0, false).await;
    common::mark_read(&state, &admin.id, &c1).await;
    let token = common::token_for(&admin);
    let app = arrgh_server::api::router(state);

    let (_, body) = send(&app, "GET", &format!("/api/titles/{t}"), Some(&token), None).await;
    assert_eq!(body["total_chapters"], 3);
    assert_eq!(body["downloaded_chapters"], 2);
    assert_eq!(body["chapters_read"], 1);
}

#[tokio::test]
async fn get_title_is_local_false_when_has_sources() {
    let state = common::build_state().await;
    let admin = common::seed_user(&state, "admin", "admin", true).await;
    let t = common::seed_title(&state, "Naruto", false).await;
    common::seed_user_title(&state, &admin.id, &t).await;
    common::add_title_source(&state, &t, "mangadex").await;
    let token = common::token_for(&admin);
    let app = arrgh_server::api::router(state);

    let (_, body) = send(&app, "GET", &format!("/api/titles/{t}"), Some(&token), None).await;
    assert_eq!(body["is_local"], false);
}

#[tokio::test]
async fn get_title_has_sync_warnings_when_warning_exists() {
    let state = common::build_state().await;
    let admin = common::seed_user(&state, "admin", "admin", true).await;
    let t = common::seed_title(&state, "Naruto", false).await;
    common::seed_user_title(&state, &admin.id, &t).await;
    common::add_sync_warning(&state, &t).await;
    let token = common::token_for(&admin);
    let app = arrgh_server::api::router(state);

    let (_, body) = send(&app, "GET", &format!("/api/titles/{t}"), Some(&token), None).await;
    assert_eq!(body["has_sync_warnings"], true);
}

// ── GET /api/titles/new-releases ─────────────────────────────────────────

#[tokio::test]
async fn new_releases_returns_new_chapters() {
    let state = common::build_state().await;
    let admin = common::seed_user(&state, "admin", "admin", true).await;
    let t = common::seed_title(&state, "Naruto", false).await;
    common::seed_user_title(&state, &admin.id, &t).await;
    let c = common::seed_chapter(&state, &t, 1.0, false).await;
    common::mark_new(&state, &c).await;
    let token = common::token_for(&admin);
    let app = arrgh_server::api::router(state);

    let (_, body) = send(&app, "GET", "/api/titles/new-releases", Some(&token), None).await;
    assert_eq!(body.as_array().unwrap().len(), 1);
    assert_eq!(body[0]["chapter_id"], c);
}

// ── DELETE /api/titles/{id} ───────────────────────────────────────────────

#[tokio::test]
async fn remove_title_no_content_when_owned() {
    let state = common::build_state().await;
    let admin = common::seed_user(&state, "admin", "admin", true).await;
    let t = common::seed_title(&state, "Naruto", false).await;
    common::seed_user_title(&state, &admin.id, &t).await;
    let token = common::token_for(&admin);
    let app = arrgh_server::api::router(state);

    let (status, _) = send(
        &app,
        "DELETE",
        &format!("/api/titles/{t}"),
        Some(&token),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::NO_CONTENT);
}

#[tokio::test]
async fn remove_title_not_found_when_not_owned() {
    let state = common::build_state().await;
    let admin = common::seed_user(&state, "admin", "admin", true).await;
    let token = common::token_for(&admin);
    let app = arrgh_server::api::router(state);

    let (status, _) = send(&app, "DELETE", "/api/titles/ghost", Some(&token), None).await;
    assert_eq!(status, StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn remove_title_does_not_delete_title_when_other_user_still_has_it() {
    let state = common::build_state().await;
    let user1 = common::seed_user(&state, "admin", "admin", true).await;
    let user2 = common::seed_user(&state, "member1", "member", false).await;
    let t = common::seed_title(&state, "Naruto", false).await;
    common::seed_user_title(&state, &user1.id, &t).await;
    common::seed_user_title(&state, &user2.id, &t).await;
    let token = common::token_for(&user1);
    let app = arrgh_server::api::router(state.clone());

    send(
        &app,
        "DELETE",
        &format!("/api/titles/{t}"),
        Some(&token),
        None,
    )
    .await;

    let still_there: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM titles WHERE id = ?")
        .bind(&t)
        .fetch_one(&state.db)
        .await
        .unwrap();
    assert_eq!(still_there, 1);
}

#[tokio::test]
async fn remove_title_deletes_title_when_last_user() {
    let state = common::build_state().await;
    let admin = common::seed_user(&state, "admin", "admin", true).await;
    let t = common::seed_title(&state, "Naruto", false).await;
    common::seed_user_title(&state, &admin.id, &t).await;
    let token = common::token_for(&admin);
    let app = arrgh_server::api::router(state.clone());

    send(
        &app,
        "DELETE",
        &format!("/api/titles/{t}"),
        Some(&token),
        None,
    )
    .await;

    let gone: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM titles WHERE id = ?")
        .bind(&t)
        .fetch_one(&state.db)
        .await
        .unwrap();
    assert_eq!(gone, 0);
}

// ── PATCH /api/titles/{id} ────────────────────────────────────────────────

#[tokio::test]
async fn patch_title_updates_auto_download() {
    let state = common::build_state().await;
    let admin = common::seed_user(&state, "admin", "admin", true).await;
    let t = common::seed_title(&state, "Naruto", false).await;
    common::seed_user_title(&state, &admin.id, &t).await;
    let token = common::token_for(&admin);
    let app = arrgh_server::api::router(state);

    let (status, _) = send(
        &app,
        "PATCH",
        &format!("/api/titles/{t}"),
        Some(&token),
        Some(json!({ "auto_download": true })),
    )
    .await;
    assert_eq!(status, StatusCode::NO_CONTENT);

    let (_, body) = send(&app, "GET", &format!("/api/titles/{t}"), Some(&token), None).await;
    assert_eq!(body["auto_download"], true);
}

#[tokio::test]
async fn patch_title_forbidden_is_explicit_for_member() {
    let state = common::build_state().await;
    let member = common::seed_user(&state, "member1", "member", false).await;
    let t = common::seed_title(&state, "Naruto", false).await;
    common::seed_user_title(&state, &member.id, &t).await;
    let token = common::token_for(&member);
    let app = arrgh_server::api::router(state);

    let (status, _) = send(
        &app,
        "PATCH",
        &format!("/api/titles/{t}"),
        Some(&token),
        Some(json!({ "is_explicit": true })),
    )
    .await;
    assert_eq!(status, StatusCode::FORBIDDEN);
}

#[tokio::test]
async fn patch_title_unprocessable_invalid_reader_mode() {
    let state = common::build_state().await;
    let admin = common::seed_user(&state, "admin", "admin", true).await;
    let t = common::seed_title(&state, "Naruto", false).await;
    common::seed_user_title(&state, &admin.id, &t).await;
    let token = common::token_for(&admin);
    let app = arrgh_server::api::router(state);

    let (status, _) = send(
        &app,
        "PATCH",
        &format!("/api/titles/{t}"),
        Some(&token),
        Some(json!({ "reader_mode": "invalid" })),
    )
    .await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
}

#[tokio::test]
async fn patch_title_sets_reader_mode() {
    let state = common::build_state().await;
    let admin = common::seed_user(&state, "admin", "admin", true).await;
    let t = common::seed_title(&state, "Naruto", false).await;
    common::seed_user_title(&state, &admin.id, &t).await;
    let token = common::token_for(&admin);
    let app = arrgh_server::api::router(state);

    send(
        &app,
        "PATCH",
        &format!("/api/titles/{t}"),
        Some(&token),
        Some(json!({ "reader_mode": "scroll" })),
    )
    .await;

    let (_, body) = send(&app, "GET", &format!("/api/titles/{t}"), Some(&token), None).await;
    assert_eq!(body["reader_mode"], "scroll");
}

#[tokio::test]
async fn patch_title_admin_can_set_explicit() {
    let state = common::build_state().await;
    let admin = common::seed_user(&state, "admin", "admin", true).await;
    let t = common::seed_title(&state, "Naruto", false).await;
    common::seed_user_title(&state, &admin.id, &t).await;
    let token = common::token_for(&admin);
    let app = arrgh_server::api::router(state);

    let (status, _) = send(
        &app,
        "PATCH",
        &format!("/api/titles/{t}"),
        Some(&token),
        Some(json!({ "is_explicit": true })),
    )
    .await;
    assert_eq!(status, StatusCode::NO_CONTENT);

    let (_, body) = send(&app, "GET", &format!("/api/titles/{t}"), Some(&token), None).await;
    assert_eq!(body["is_explicit"], true);
}

#[tokio::test]
async fn patch_title_unprocessable_invalid_content_type() {
    let state = common::build_state().await;
    let admin = common::seed_user(&state, "admin", "admin", true).await;
    let t = common::seed_title(&state, "Naruto", false).await;
    common::seed_user_title(&state, &admin.id, &t).await;
    let token = common::token_for(&admin);
    let app = arrgh_server::api::router(state);

    let (status, _) = send(
        &app,
        "PATCH",
        &format!("/api/titles/{t}"),
        Some(&token),
        Some(json!({ "content_type": "comic" })),
    )
    .await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
}

#[tokio::test]
async fn patch_title_not_found_when_not_owned() {
    let state = common::build_state().await;
    let admin = common::seed_user(&state, "admin", "admin", true).await;
    let token = common::token_for(&admin);
    let app = arrgh_server::api::router(state);

    let (status, _) = send(
        &app,
        "PATCH",
        "/api/titles/ghost",
        Some(&token),
        Some(json!({ "auto_download": true })),
    )
    .await;
    assert_eq!(status, StatusCode::NOT_FOUND);
}

// ── POST /api/titles/{id}/sync ────────────────────────────────────────────

#[tokio::test]
async fn sync_title_accepted_when_source_links_exist() {
    let state = common::build_state().await;
    let admin = common::seed_user(&state, "admin", "admin", true).await;
    let t = common::seed_title(&state, "Naruto", false).await;
    common::seed_user_title(&state, &admin.id, &t).await;
    common::add_title_source(&state, &t, "mangadex").await;
    let token = common::token_for(&admin);
    let app = arrgh_server::api::router(state);

    let (status, _) = send(
        &app,
        "POST",
        &format!("/api/titles/{t}/sync"),
        Some(&token),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::ACCEPTED);
}

#[tokio::test]
async fn sync_title_not_found_when_no_source_links() {
    let state = common::build_state().await;
    let admin = common::seed_user(&state, "admin", "admin", true).await;
    let t = common::seed_title(&state, "Naruto", false).await;
    common::seed_user_title(&state, &admin.id, &t).await;
    let token = common::token_for(&admin);
    let app = arrgh_server::api::router(state);

    let (status, _) = send(
        &app,
        "POST",
        &format!("/api/titles/{t}/sync"),
        Some(&token),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn sync_title_not_found_when_not_owned() {
    let state = common::build_state().await;
    let admin = common::seed_user(&state, "admin", "admin", true).await;
    let token = common::token_for(&admin);
    let app = arrgh_server::api::router(state);

    let (status, _) = send(&app, "POST", "/api/titles/ghost/sync", Some(&token), None).await;
    assert_eq!(status, StatusCode::NOT_FOUND);
}

// ── GET /api/titles/{id}/sync-log ─────────────────────────────────────────

#[tokio::test]
async fn get_sync_log_returns_entries() {
    let state = common::build_state().await;
    let admin = common::seed_user(&state, "admin", "admin", true).await;
    let t = common::seed_title(&state, "Naruto", false).await;
    common::seed_user_title(&state, &admin.id, &t).await;
    common::add_sync_log(&state, &t, "Syncing chapters from mangadex…").await;
    common::add_sync_log(&state, &t, "Synced 12 chapters from mangadex").await;
    let token = common::token_for(&admin);
    let app = arrgh_server::api::router(state);

    let (_, body) = send(
        &app,
        "GET",
        &format!("/api/titles/{t}/sync-log"),
        Some(&token),
        None,
    )
    .await;
    assert_eq!(body.as_array().unwrap().len(), 2);
}

#[tokio::test]
async fn get_sync_log_not_found_when_not_owned() {
    let state = common::build_state().await;
    let admin = common::seed_user(&state, "admin", "admin", true).await;
    let token = common::token_for(&admin);
    let app = arrgh_server::api::router(state);

    let (status, _) = send(
        &app,
        "GET",
        "/api/titles/ghost/sync-log",
        Some(&token),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::NOT_FOUND);
}

// ── POST /api/titles/{id}/refresh-metadata ────────────────────────────────

#[tokio::test]
async fn refresh_metadata_returns_404_when_title_not_owned() {
    let state = common::build_state().await;
    let admin = common::seed_user(&state, "admin", "admin", true).await;
    let token = common::token_for(&admin);
    let app = arrgh_server::api::router(state);

    let (status, _) = send(
        &app,
        "POST",
        "/api/titles/nonexistent/refresh-metadata",
        Some(&token),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn refresh_metadata_returns_401_without_token() {
    let app = arrgh_server::api::router(common::build_state().await);
    let (status, _) = send(&app, "POST", "/api/titles/any/refresh-metadata", None, None).await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn refresh_metadata_clears_sync_warnings_and_returns_202() {
    let state = common::build_state().await;
    let admin = common::seed_user(&state, "admin", "admin", true).await;
    let t = common::seed_title(&state, "Naruto", false).await;
    common::seed_user_title(&state, &admin.id, &t).await;
    common::add_sync_warning(&state, &t).await;
    let token = common::token_for(&admin);
    let app = arrgh_server::api::router(state.clone());

    let (status, _) = send(
        &app,
        "POST",
        &format!("/api/titles/{t}/refresh-metadata"),
        Some(&token),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::ACCEPTED);

    let warnings: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM sync_warnings WHERE title_id = ?")
        .bind(&t)
        .fetch_one(&state.db)
        .await
        .unwrap();
    assert_eq!(warnings, 0);
}
