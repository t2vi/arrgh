//! Shared integration-test helpers. Each test file that `mod common;`s this
//! gets its own isolated temp-file SQLite DB per `build_state()` call.
//!
//! `mod common;` is recompiled per test binary (version.rs, logs.rs,
//! auth.rs, ...) — a helper only some of them use isn't dead code overall.
#![allow(dead_code)]

use arrgh_server::config::Config;
use arrgh_server::logs::LogBuffer;
use arrgh_server::state::{connect_db, AppState};

pub const JWT_SECRET: &str = "integration-test-secret-32chars!";

pub async fn build_state() -> AppState {
    build_state_with_plugin_host("http://localhost:4000").await
}

pub async fn build_state_with_plugin_host(plugin_host_url: &str) -> AppState {
    build_state_with(|c| c.plugin_host_url = plugin_host_url.to_string()).await
}

/// Points every Discover metadata-authority URL (S6 #128) at one mock
/// server's base URL — good enough for tests that only care about one or
/// two authorities succeeding; the rest simply have nothing to match and
/// fail closed via `SafeSearch`-equivalent handling.
pub async fn build_discover_state(mock_url: &str) -> AppState {
    build_state_with(|c| {
        c.plugin_host_url = mock_url.to_string();
        c.mangaupdates_url = mock_url.to_string();
        c.anilist_url = mock_url.to_string();
        c.mangadex_meta_url = mock_url.to_string();
        c.wuxiaworld_meta_url = mock_url.to_string();
    })
    .await
}

/// Points `plugin_host_url` + `download_dir` at test-local values (S7 #129
/// downloader tests — need a mock plugin-host and a throwaway download dir).
pub async fn build_downloader_state(plugin_host_url: &str, download_dir: &str) -> AppState {
    build_state_with(|c| {
        c.plugin_host_url = plugin_host_url.to_string();
        c.download_dir = download_dir.to_string();
    })
    .await
}

/// Points `plugin_index_url` + `plugin_host_url` at test-local values (S9
/// #131 plugin install/delete tests).
pub async fn build_plugins_state(plugin_index_url: &str, plugin_host_url: &str) -> AppState {
    build_state_with(|c| {
        c.plugin_index_url = plugin_index_url.to_string();
        c.plugin_host_url = plugin_host_url.to_string();
    })
    .await
}

async fn build_state_with(configure: impl FnOnce(&mut Config)) -> AppState {
    let db_path = std::env::temp_dir().join(format!("arrgh-rust-test-{}.db", uuid::Uuid::new_v4()));
    let db_path = db_path.to_str().unwrap().to_string();

    let mut config = Config {
        jwt_secret: Some(JWT_SECRET.into()),
        database_path: db_path.clone(),
        ..Config::from_env().expect("default config")
    };
    configure(&mut config);

    // connect_db runs server/migrations/ itself as of S10 (#132) — no
    // manual schema setup needed here anymore.
    let db = connect_db(&db_path).await.expect("connect test db");

    AppState::new(config, LogBuffer::new("info"), db)
}

/// Spins up a throwaway HTTP server that always answers with `body` (200) or
/// a 500 when `fail` is set — stands in for plugin-host's chapters endpoint
/// in chapter-sync tests. Returns its base URL. Outlives the test (detached
/// task on an isolated ephemeral port); the process exits at test-binary end.
pub async fn start_mock_plugin_host(body: &'static str, fail: bool) -> String {
    use axum::http::StatusCode;
    use axum::response::IntoResponse;

    let app = axum::Router::new().fallback(move || async move {
        if fail {
            (StatusCode::INTERNAL_SERVER_ERROR, "boom").into_response()
        } else {
            (StatusCode::OK, [("content-type", "application/json")], body).into_response()
        }
    });
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    tokio::spawn(async move {
        axum::serve(listener, app).await.unwrap();
    });
    format!("http://{addr}")
}

/// Like `start_mock_plugin_host` but path-routes `/{source}/search` and
/// `/{source}/manga/{id}/chapters` to different canned bodies — needed for
/// `discover::match_sources` (S6 #128), which hits both shapes in one flow.
pub async fn start_mock_source_match_host(
    search_body: &'static str,
    chapters_body: &'static str,
) -> String {
    use axum::response::IntoResponse;

    let app = axum::Router::new()
        .route(
            "/{source}/search",
            axum::routing::get(move || async move {
                ([("content-type", "application/json")], search_body).into_response()
            }),
        )
        .route(
            "/{source}/manga/{id}/chapters",
            axum::routing::get(move || async move {
                ([("content-type", "application/json")], chapters_body).into_response()
            }),
        );
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    tokio::spawn(async move {
        axum::serve(listener, app).await.unwrap();
    });
    format!("http://{addr}")
}

/// Inserts a user directly (bypassing the API) for tests that need one to
/// already exist. Fixed password `"password123"` — mirrors the .NET
/// `Fake.AdminUser`/`Fake.MemberUser` fixtures.
pub async fn seed_user(
    state: &AppState,
    username: &str,
    role: &str,
    allow_explicit: bool,
) -> arrgh_server::users::UserRow {
    let id = uuid::Uuid::new_v4().to_string();
    let hash = arrgh_server::auth::hash_password("password123").unwrap();
    arrgh_server::users::insert(&state.db, &id, username, &hash, role, allow_explicit)
        .await
        .unwrap();
    arrgh_server::users::find_by_id(&state.db, &id)
        .await
        .unwrap()
        .unwrap()
}

pub fn token_for(user: &arrgh_server::users::UserRow) -> String {
    arrgh_server::auth::create_token(
        &user.id,
        &user.username,
        &user.role,
        user.allow_explicit,
        JWT_SECRET,
    )
    .unwrap()
}

/// Timestamp for seeded rows — monotonic enough across sequential awaited
/// inserts within one test for ORDER BY created_at to be deterministic.
fn now_str() -> String {
    use time::macros::format_description;
    time::OffsetDateTime::now_utc()
        .format(&format_description!(
            "[year]-[month]-[day] [hour]:[minute]:[second].[subsecond digits:6]"
        ))
        .unwrap()
}

/// Inserts a title row directly, returning its id. Defaults: status
/// "ongoing", sync_status "ready", content_type "manga", not explicit.
pub async fn seed_title(state: &AppState, title: &str, is_explicit: bool) -> String {
    let id = uuid::Uuid::new_v4().to_string();
    let now = now_str();
    sqlx::query(
        "INSERT INTO titles (id, title, status, created_at, updated_at, sync_status, content_type, is_explicit) \
         VALUES (?, ?, 'ongoing', ?, ?, 'ready', 'manga', ?)",
    )
    .bind(&id)
    .bind(title)
    .bind(&now)
    .bind(&now)
    .bind(is_explicit)
    .execute(&state.db)
    .await
    .unwrap();
    id
}

pub async fn set_mangaupdates_id(state: &AppState, title_id: &str, mu_id: &str) {
    sqlx::query("UPDATE titles SET mangaupdates_id = ? WHERE id = ?")
        .bind(mu_id)
        .bind(title_id)
        .execute(&state.db)
        .await
        .unwrap();
}

pub async fn set_metadata_source(state: &AppState, title_id: &str, source: &str, source_id: &str) {
    sqlx::query("UPDATE titles SET metadata_source = ?, metadata_source_id = ? WHERE id = ?")
        .bind(source)
        .bind(source_id)
        .bind(title_id)
        .execute(&state.db)
        .await
        .unwrap();
}

/// Adds `user_id` as an owner of `title_id` (`user_titles` row).
pub async fn seed_user_title(state: &AppState, user_id: &str, title_id: &str) {
    sqlx::query("INSERT INTO user_titles (user_id, title_id, added_at) VALUES (?, ?, ?)")
        .bind(user_id)
        .bind(title_id)
        .bind(now_str())
        .execute(&state.db)
        .await
        .unwrap();
}

/// Inserts a chapter row, returning its id. `downloaded` defaults false.
pub async fn seed_chapter(
    state: &AppState,
    title_id: &str,
    number: f64,
    downloaded: bool,
) -> String {
    let id = uuid::Uuid::new_v4().to_string();
    sqlx::query(
        "INSERT INTO chapters (id, title_id, number, page_count, downloaded, created_at, is_new, chapter_format) \
         VALUES (?, ?, ?, 0, ?, ?, 0, 'cbz')",
    )
    .bind(&id)
    .bind(title_id)
    .bind(number)
    .bind(downloaded)
    .bind(now_str())
    .execute(&state.db)
    .await
    .unwrap();
    id
}

pub async fn mark_new(state: &AppState, chapter_id: &str) {
    sqlx::query("UPDATE chapters SET is_new = 1 WHERE id = ?")
        .bind(chapter_id)
        .execute(&state.db)
        .await
        .unwrap();
}

pub async fn mark_read(state: &AppState, user_id: &str, chapter_id: &str) {
    sqlx::query(
        "INSERT INTO read_progress (id, user_id, chapter_id, current_page, completed, updated_at) \
         VALUES (?, ?, ?, 0, 1, ?)",
    )
    .bind(uuid::Uuid::new_v4().to_string())
    .bind(user_id)
    .bind(chapter_id)
    .bind(now_str())
    .execute(&state.db)
    .await
    .unwrap();
}

pub async fn add_chapter_source(state: &AppState, chapter_id: &str, source: &str) {
    sqlx::query(
        "INSERT INTO chapter_sources (id, chapter_id, source, source_id) VALUES (?, ?, ?, ?)",
    )
    .bind(uuid::Uuid::new_v4().to_string())
    .bind(chapter_id)
    .bind(source)
    .bind(uuid::Uuid::new_v4().to_string())
    .execute(&state.db)
    .await
    .unwrap();
}

/// Seeds an errored `download_queue` row for `chapter_id`, for the
/// re-queue-on-retry test.
pub async fn seed_errored_queue_item(
    state: &AppState,
    chapter_id: &str,
    manga_title: &str,
    chapter_num: f64,
) {
    let now = now_str();
    sqlx::query(
        "INSERT INTO download_queue \
             (id, chapter_id, manga_title, chapter_num, status, error, created_at, updated_at, pages_downloaded, pages_total) \
         VALUES (?, ?, ?, ?, 'error', 'timeout', ?, ?, 0, 0)",
    )
    .bind(uuid::Uuid::new_v4().to_string())
    .bind(chapter_id)
    .bind(manga_title)
    .bind(chapter_num)
    .bind(&now)
    .bind(&now)
    .execute(&state.db)
    .await
    .unwrap();
}

pub async fn add_title_source(state: &AppState, title_id: &str, source: &str) {
    sqlx::query(
        "INSERT INTO title_sources (id, title_id, source, source_id, discovered_at) VALUES (?, ?, ?, 'src-1', ?)",
    )
    .bind(uuid::Uuid::new_v4().to_string())
    .bind(title_id)
    .bind(source)
    .bind(now_str())
    .execute(&state.db)
    .await
    .unwrap();
}

pub async fn add_sync_warning(state: &AppState, title_id: &str) {
    sqlx::query(
        "INSERT INTO sync_warnings (id, title_id, plugin_id, message, created_at) VALUES (?, ?, 'plugin-1', 'warn', ?)",
    )
    .bind(uuid::Uuid::new_v4().to_string())
    .bind(title_id)
    .bind(now_str())
    .execute(&state.db)
    .await
    .unwrap();
}

pub async fn add_sync_log(state: &AppState, title_id: &str, message: &str) {
    sqlx::query("INSERT INTO sync_log (id, title_id, message, created_at) VALUES (?, ?, ?, ?)")
        .bind(uuid::Uuid::new_v4().to_string())
        .bind(title_id)
        .bind(message)
        .bind(now_str())
        .execute(&state.db)
        .await
        .unwrap();
}

/// Like `seed_source` but sets `source_key` too — required for Discover's
/// `match_sources` (S6 #128), which only considers sources with a non-null
/// `source_key`.
pub async fn seed_source_with_key(
    state: &AppState,
    name: &str,
    source_key: &str,
    content_types: &str,
) -> String {
    let id = uuid::Uuid::new_v4().to_string();
    sqlx::query(
        "INSERT INTO external_sources (id, name, base_url, content_types, enabled, created_at, is_community, priority, source_key, default_explicit) \
         VALUES (?, ?, 'http://example.com', ?, 1, datetime('now'), 0, 100, ?, 0)",
    )
    .bind(&id)
    .bind(name)
    .bind(content_types)
    .bind(source_key)
    .execute(&state.db)
    .await
    .unwrap();
    id
}

/// Inserts a `download_queue` row with an arbitrary status/owner, returning
/// its id — for S7 (#129) queue-management tests (cancel/remove/ownership).
pub async fn seed_queue_item(
    state: &AppState,
    chapter_id: &str,
    manga_title: &str,
    chapter_num: f64,
    status: &str,
    queued_by: Option<&str>,
) -> String {
    let id = uuid::Uuid::new_v4().to_string();
    let now = now_str();
    sqlx::query(
        "INSERT INTO download_queue \
             (id, chapter_id, manga_title, chapter_num, status, created_at, updated_at, pages_downloaded, pages_total, queued_by) \
         VALUES (?, ?, ?, ?, ?, ?, ?, 0, 0, ?)",
    )
    .bind(&id)
    .bind(chapter_id)
    .bind(manga_title)
    .bind(chapter_num)
    .bind(status)
    .bind(&now)
    .bind(&now)
    .bind(queued_by)
    .execute(&state.db)
    .await
    .unwrap();
    id
}

/// Inserts an external source row directly, returning its id.
pub async fn seed_source(
    state: &AppState,
    name: &str,
    base_url: &str,
    content_types: &str,
    api_key: Option<&str>,
) -> String {
    let id = uuid::Uuid::new_v4().to_string();
    sqlx::query(
        "INSERT INTO external_sources (id, name, base_url, api_key, content_types, enabled, created_at, is_community, priority, default_explicit) VALUES (?, ?, ?, ?, ?, 1, datetime('now'), 0, 100, 0)",
    )
    .bind(&id)
    .bind(name)
    .bind(base_url)
    .bind(api_key)
    .bind(content_types)
    .execute(&state.db)
    .await
    .unwrap();
    id
}
