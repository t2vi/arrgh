//! Scheduled re-sync + auto-download (GH #200, spec 007 FR-001a).
//!
//! Each test seeds a library title that already has chapter 1, points it at a
//! mock plugin-host that now reports chapters 1 and 2, runs one scheduler tick
//! and checks what got queued. Only chapters the sync *newly finds* are
//! auto-downloaded — never the existing backlog.

mod common;

use std::time::Duration;

use arrgh_server::scheduler;
use arrgh_server::state::AppState;

const TWO_CHAPTERS: &str = r#"[
  {"source_id":"src-ch-1","id":"src-ch-1","number":1.0,"title":"Chapter 1"},
  {"source_id":"src-ch-2","id":"src-ch-2","number":2.0,"title":"Chapter 2"}
]"#;

/// Library title with chapter 1 already synced; returns (state, title_id).
async fn library_title(title_auto: Option<bool>, global_auto: Option<bool>) -> (AppState, String) {
    let url = common::start_mock_plugin_host(TWO_CHAPTERS, false).await;
    let state = common::build_state_with_plugin_host(&url).await;
    let user = common::seed_user(&state, "reader", "member", false).await;
    let t = common::seed_title(&state, "Solo Leveling", false).await;
    common::seed_user_title(&state, &user.id, &t).await;
    common::add_title_source(&state, &t, "mangadex").await;
    let c1 = common::seed_chapter(&state, &t, 1.0, false).await;
    common::add_chapter_source(&state, &c1, "mangadex").await;
    sqlx::query("UPDATE titles SET auto_download = ? WHERE id = ?")
        .bind(title_auto)
        .bind(&t)
        .execute(&state.db)
        .await
        .unwrap();
    if let Some(g) = global_auto {
        arrgh_server::settings::set(
            &state.db,
            arrgh_server::settings::AUTO_DOWNLOAD,
            &g.to_string(),
        )
        .await
        .unwrap();
    }
    (state, t)
}

async fn tick(state: &AppState) {
    scheduler::tick(&state.db, &state.http, &state.config.plugin_host_url).await;
}

async fn queued_numbers(state: &AppState) -> Vec<f64> {
    sqlx::query_scalar("SELECT chapter_num FROM download_queue ORDER BY chapter_num")
        .fetch_all(&state.db)
        .await
        .unwrap()
}

async fn chapter_count(state: &AppState, title_id: &str) -> i64 {
    sqlx::query_scalar("SELECT COUNT(*) FROM chapters WHERE title_id = ?")
        .bind(title_id)
        .fetch_one(&state.db)
        .await
        .unwrap()
}

// spec: 007/FR-001a
#[tokio::test]
async fn per_title_always_queues_only_newly_found_chapters() {
    let (state, t) = library_title(Some(true), Some(false)).await;
    tick(&state).await;
    assert_eq!(
        chapter_count(&state, &t).await,
        2,
        "tick re-synced the title"
    );
    assert_eq!(
        queued_numbers(&state).await,
        vec![2.0],
        "backlog (ch 1) not queued"
    );
    let status: String = sqlx::query_scalar("SELECT sync_status FROM titles WHERE id = ?")
        .bind(&t)
        .fetch_one(&state.db)
        .await
        .unwrap();
    assert_eq!(status, "ready");
}

// spec: 007/FR-001a
#[tokio::test]
async fn per_title_default_follows_global_setting() {
    let (state, _) = library_title(None, Some(true)).await;
    tick(&state).await;
    assert_eq!(queued_numbers(&state).await, vec![2.0]);
}

// spec: 007/FR-001a
#[tokio::test]
async fn per_title_never_overrides_global_on() {
    let (state, t) = library_title(Some(false), Some(true)).await;
    tick(&state).await;
    assert_eq!(chapter_count(&state, &t).await, 2, "still synced");
    assert!(queued_numbers(&state).await.is_empty());
}

// spec: 007/FR-001a
#[tokio::test]
async fn global_default_is_off() {
    let (state, t) = library_title(None, None).await;
    tick(&state).await;
    assert_eq!(chapter_count(&state, &t).await, 2);
    assert!(queued_numbers(&state).await.is_empty());
}

// spec: 007/FR-001a
#[tokio::test]
async fn interval_comes_from_index_interval_hours() {
    let state = common::build_state().await;
    assert_eq!(
        scheduler::interval(&state.db).await,
        Duration::from_secs(6 * 3600)
    );
    arrgh_server::settings::set(&state.db, arrgh_server::settings::INDEX_INTERVAL_HOURS, "2")
        .await
        .unwrap();
    assert_eq!(
        scheduler::interval(&state.db).await,
        Duration::from_secs(2 * 3600)
    );
    arrgh_server::settings::set(&state.db, arrgh_server::settings::INDEX_INTERVAL_HOURS, "0")
        .await
        .unwrap();
    assert_eq!(
        scheduler::interval(&state.db).await,
        Duration::from_secs(3600),
        "clamped to ≥1h"
    );
}
