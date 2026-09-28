//! `/api/backups/*` — DB backup + restore (spec 035, issue #251).

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

// ── US1: manual backup ───────────────────────────────────────────────────

// spec: 035/FR-001
#[tokio::test]
async fn backup_now_with_configured_destination_creates_a_listed_file() {
    let state = common::build_state().await;
    let admin = common::seed_user(&state, "admin", "admin", true).await;
    let token = common::token_for(&admin);
    let dir = std::env::temp_dir().join(format!("arrgh-backup-test-{}", uuid::Uuid::new_v4()));
    arrgh_server::settings::set(
        &state.db,
        arrgh_server::settings::BACKUP_DIR,
        dir.to_str().unwrap(),
    )
    .await
    .unwrap();
    let app = arrgh_server::api::router(state);

    let (status, body) = send(&app, "POST", "/api/backups", Some(&token)).await;
    assert_eq!(status, StatusCode::CREATED);
    let filename = body["filename"].as_str().unwrap().to_string();
    assert!(dir.join(&filename).exists());

    let (status, body) = send(&app, "GET", "/api/backups", Some(&token)).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body.as_array().unwrap().len(), 1);
    assert_eq!(body[0]["filename"], filename);
}

// spec: 035/FR-001
#[tokio::test]
async fn backup_now_with_no_destination_is_refused() {
    let state = common::build_state().await;
    let admin = common::seed_user(&state, "admin", "admin", true).await;
    let token = common::token_for(&admin);
    let app = arrgh_server::api::router(state);

    let (status, _) = send(&app, "POST", "/api/backups", Some(&token)).await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
}

// spec: 035/FR-005
#[tokio::test]
async fn list_with_no_destination_configured_is_empty_not_an_error() {
    let state = common::build_state().await;
    let admin = common::seed_user(&state, "admin", "admin", true).await;
    let token = common::token_for(&admin);
    let app = arrgh_server::api::router(state);

    let (status, body) = send(&app, "GET", "/api/backups", Some(&token)).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body, Value::Array(vec![]));
}

// spec: 035/FR-012
#[tokio::test]
async fn backups_routes_are_admin_gated() {
    let state = common::build_state().await;
    let member = common::seed_user(&state, "member1", "member", false).await;
    let member_token = common::token_for(&member);
    let app = arrgh_server::api::router(state);

    let (status, _) = send(&app, "GET", "/api/backups", None).await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);

    let (status, _) = send(&app, "GET", "/api/backups", Some(&member_token)).await;
    assert_eq!(status, StatusCode::FORBIDDEN);

    let (status, _) = send(&app, "POST", "/api/backups", Some(&member_token)).await;
    assert_eq!(status, StatusCode::FORBIDDEN);
}

// ── US2: restore from a listed backup ────────────────────────────────────

// spec: 035/FR-007, 035/FR-011, 035/FR-013
#[tokio::test]
async fn restore_from_a_valid_listed_backup_swaps_in_the_file() {
    let state = common::build_state().await;
    let admin = common::seed_user(&state, "admin", "admin", true).await;
    let token = common::token_for(&admin);
    let dir = std::env::temp_dir().join(format!("arrgh-backup-test-{}", uuid::Uuid::new_v4()));
    arrgh_server::settings::set(
        &state.db,
        arrgh_server::settings::BACKUP_DIR,
        dir.to_str().unwrap(),
    )
    .await
    .unwrap();
    let app = arrgh_server::api::router(state.clone());

    let (_, body) = send(&app, "POST", "/api/backups", Some(&token)).await;
    let filename = body["filename"].as_str().unwrap().to_string();
    let backup_bytes_before = tokio::fs::read(dir.join(&filename)).await.unwrap();

    let (status, _) = send(
        &app,
        "POST",
        &format!("/api/backups/{filename}/restore"),
        Some(&token),
    )
    .await;
    assert_eq!(status, StatusCode::ACCEPTED);

    // restart_on_restore is false in tests (common::build_state's seam) — the live DB file was
    // swapped in-place without the process actually exiting, so we can assert on it directly.
    let live_bytes = tokio::fs::read(&state.config.database_path).await.unwrap();
    assert_eq!(live_bytes, backup_bytes_before);
}

// spec: 035/FR-009
#[tokio::test]
async fn restore_rejects_a_backup_newer_than_the_running_app_and_leaves_live_db_untouched() {
    let state = common::build_state().await;
    let admin = common::seed_user(&state, "admin", "admin", true).await;
    let token = common::token_for(&admin);
    let dir = std::env::temp_dir().join(format!("arrgh-backup-test-{}", uuid::Uuid::new_v4()));
    tokio::fs::create_dir_all(&dir).await.unwrap();
    let filename = "arrgh-backup-2099-01-01T00-00-00.db";
    let candidate_path = dir.join(filename);

    // A fixture DB whose _sqlx_migrations claims a version far beyond anything this binary knows.
    let candidate_pool = sqlx::sqlite::SqlitePoolOptions::new()
        .connect_with(
            sqlx::sqlite::SqliteConnectOptions::new()
                .filename(&candidate_path)
                .create_if_missing(true),
        )
        .await
        .unwrap();
    sqlx::query(
        "CREATE TABLE _sqlx_migrations (version BIGINT PRIMARY KEY, description TEXT, \
         installed_on TIMESTAMP, success BOOLEAN, checksum BLOB, execution_time BIGINT)",
    )
    .execute(&candidate_pool)
    .await
    .unwrap();
    sqlx::query(
        "INSERT INTO _sqlx_migrations VALUES (9999999999, 'future', datetime('now'), 1, x'00', 0)",
    )
    .execute(&candidate_pool)
    .await
    .unwrap();
    candidate_pool.close().await;

    arrgh_server::settings::set(
        &state.db,
        arrgh_server::settings::BACKUP_DIR,
        dir.to_str().unwrap(),
    )
    .await
    .unwrap();
    let live_bytes_before = tokio::fs::read(&state.config.database_path).await.unwrap();
    let app = arrgh_server::api::router(state.clone());

    let (status, body) = send(
        &app,
        "POST",
        &format!("/api/backups/{filename}/restore"),
        Some(&token),
    )
    .await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
    assert_eq!(body["error"], "backup_too_new");

    let live_bytes_after = tokio::fs::read(&state.config.database_path).await.unwrap();
    assert_eq!(live_bytes_before, live_bytes_after);
}

// spec: 035/FR-009
#[tokio::test]
async fn restore_rejects_a_non_sqlite_file() {
    let state = common::build_state().await;
    let admin = common::seed_user(&state, "admin", "admin", true).await;
    let token = common::token_for(&admin);
    let dir = std::env::temp_dir().join(format!("arrgh-backup-test-{}", uuid::Uuid::new_v4()));
    tokio::fs::create_dir_all(&dir).await.unwrap();
    let filename = "arrgh-backup-2026-01-01T00-00-00.db";
    tokio::fs::write(dir.join(filename), b"not a sqlite file")
        .await
        .unwrap();
    arrgh_server::settings::set(
        &state.db,
        arrgh_server::settings::BACKUP_DIR,
        dir.to_str().unwrap(),
    )
    .await
    .unwrap();
    let app = arrgh_server::api::router(state);

    let (status, body) = send(
        &app,
        "POST",
        &format!("/api/backups/{filename}/restore"),
        Some(&token),
    )
    .await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
    assert_eq!(body["error"], "invalid_backup");
}

// spec: 035/FR-007
#[tokio::test]
async fn restore_unknown_filename_is_404() {
    let state = common::build_state().await;
    let admin = common::seed_user(&state, "admin", "admin", true).await;
    let token = common::token_for(&admin);
    let app = arrgh_server::api::router(state);

    let (status, _) = send(
        &app,
        "POST",
        "/api/backups/arrgh-backup-2026-01-01T00-00-00.db/restore",
        Some(&token),
    )
    .await;
    assert_eq!(status, StatusCode::NOT_FOUND);
}

// ── US3: restore from an uploaded file ───────────────────────────────────

fn multipart_body(field_name: &str, filename: &str, content: &[u8]) -> (String, Vec<u8>) {
    let boundary = "arrgh-test-boundary";
    let mut body = Vec::new();
    body.extend_from_slice(
        format!(
            "--{boundary}\r\nContent-Disposition: form-data; name=\"{field_name}\"; filename=\"{filename}\"\r\nContent-Type: application/octet-stream\r\n\r\n"
        )
        .as_bytes(),
    );
    body.extend_from_slice(content);
    body.extend_from_slice(format!("\r\n--{boundary}--\r\n").as_bytes());
    (boundary.to_string(), body)
}

// spec: 035/FR-008
#[tokio::test]
async fn restore_upload_with_a_valid_backup_swaps_it_in() {
    let state = common::build_state().await;
    let admin = common::seed_user(&state, "admin", "admin", true).await;
    let token = common::token_for(&admin);
    let app = arrgh_server::api::router(state.clone());

    let backup_bytes = tokio::fs::read(&state.config.database_path).await.unwrap();
    let (boundary, body) = multipart_body("file", "mine.db", &backup_bytes);

    let req = Request::builder()
        .method("POST")
        .uri("/api/backups/restore-upload")
        .header("authorization", format!("Bearer {token}"))
        .header(
            "content-type",
            format!("multipart/form-data; boundary={boundary}"),
        )
        .body(Body::from(body))
        .unwrap();
    let res = app.clone().oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::ACCEPTED);

    let live_bytes = tokio::fs::read(&state.config.database_path).await.unwrap();
    assert_eq!(live_bytes, backup_bytes);
}

// spec: 035/FR-008, 035/FR-009
#[tokio::test]
async fn restore_upload_with_an_invalid_file_is_rejected() {
    let state = common::build_state().await;
    let admin = common::seed_user(&state, "admin", "admin", true).await;
    let token = common::token_for(&admin);
    let live_bytes_before = tokio::fs::read(&state.config.database_path).await.unwrap();
    let app = arrgh_server::api::router(state.clone());

    let (boundary, body) = multipart_body("file", "junk.db", b"not a sqlite file");
    let req = Request::builder()
        .method("POST")
        .uri("/api/backups/restore-upload")
        .header("authorization", format!("Bearer {token}"))
        .header(
            "content-type",
            format!("multipart/form-data; boundary={boundary}"),
        )
        .body(Body::from(body))
        .unwrap();
    let res = app.clone().oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::UNPROCESSABLE_ENTITY);
    let bytes = res.into_body().collect().await.unwrap().to_bytes();
    let json: Value = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(json["error"], "invalid_backup");

    let live_bytes_after = tokio::fs::read(&state.config.database_path).await.unwrap();
    assert_eq!(live_bytes_before, live_bytes_after);
}

// spec: 035/FR-012
#[tokio::test]
async fn restore_upload_unauthenticated_is_401() {
    let state = common::build_state().await;
    let app = arrgh_server::api::router(state);
    let (boundary, body) = multipart_body("file", "x.db", b"irrelevant");
    let req = Request::builder()
        .method("POST")
        .uri("/api/backups/restore-upload")
        .header(
            "content-type",
            format!("multipart/form-data; boundary={boundary}"),
        )
        .body(Body::from(body))
        .unwrap();
    let res = app.clone().oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::UNAUTHORIZED);
}

// ── US5: delete ───────────────────────────────────────────────────────────

// spec: 035/FR-006
#[tokio::test]
async fn delete_removes_from_disk_and_from_the_list() {
    let state = common::build_state().await;
    let admin = common::seed_user(&state, "admin", "admin", true).await;
    let token = common::token_for(&admin);
    let dir = std::env::temp_dir().join(format!("arrgh-backup-test-{}", uuid::Uuid::new_v4()));
    arrgh_server::settings::set(
        &state.db,
        arrgh_server::settings::BACKUP_DIR,
        dir.to_str().unwrap(),
    )
    .await
    .unwrap();
    let app = arrgh_server::api::router(state);

    let (_, body) = send(&app, "POST", "/api/backups", Some(&token)).await;
    let filename = body["filename"].as_str().unwrap().to_string();

    let (status, _) = send(
        &app,
        "DELETE",
        &format!("/api/backups/{filename}"),
        Some(&token),
    )
    .await;
    assert_eq!(status, StatusCode::NO_CONTENT);
    assert!(!dir.join(&filename).exists());

    let (_, body) = send(&app, "GET", "/api/backups", Some(&token)).await;
    assert_eq!(body.as_array().unwrap().len(), 0);
}

// spec: 035/FR-012
#[tokio::test]
async fn delete_is_admin_gated() {
    let state = common::build_state().await;
    let member = common::seed_user(&state, "member1", "member", false).await;
    let member_token = common::token_for(&member);
    let app = arrgh_server::api::router(state);

    let (status, _) = send(
        &app,
        "DELETE",
        "/api/backups/arrgh-backup-2026-01-01T00-00-00.db",
        None,
    )
    .await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);

    let (status, _) = send(
        &app,
        "DELETE",
        "/api/backups/arrgh-backup-2026-01-01T00-00-00.db",
        Some(&member_token),
    )
    .await;
    assert_eq!(status, StatusCode::FORBIDDEN);
}

// spec: 035/FR-006
#[tokio::test]
async fn delete_unknown_filename_is_404() {
    let state = common::build_state().await;
    let admin = common::seed_user(&state, "admin", "admin", true).await;
    let token = common::token_for(&admin);
    let dir = std::env::temp_dir().join(format!("arrgh-backup-test-{}", uuid::Uuid::new_v4()));
    arrgh_server::settings::set(
        &state.db,
        arrgh_server::settings::BACKUP_DIR,
        dir.to_str().unwrap(),
    )
    .await
    .unwrap();
    let app = arrgh_server::api::router(state);

    let (status, _) = send(
        &app,
        "DELETE",
        "/api/backups/arrgh-backup-2026-01-01T00-00-00.db",
        Some(&token),
    )
    .await;
    assert_eq!(status, StatusCode::NOT_FOUND);
}
