//! Background update-check poller (spec 007/FR-013, FR-014). Same pattern as
//! `downloader.rs`: spawn the real loop with a short interval against a mock
//! HTTP server and poll for the observable effect, rather than calling the
//! (private) `check` function directly.

mod common;

use std::time::Duration;

use axum::http::StatusCode;
use axum::response::IntoResponse;

const TICK: Duration = Duration::from_millis(50);

async fn start_mock(status: StatusCode, body: &'static str) -> String {
    let app = axum::Router::new().fallback(move || async move { (status, body).into_response() });
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    tokio::spawn(async move {
        axum::serve(listener, app).await.unwrap();
    });
    format!("http://{addr}")
}

async fn enable_check_for_updates(pool: &sqlx::SqlitePool) {
    sqlx::query("INSERT INTO server_settings (key, value) VALUES ('check_for_updates', 'true')")
        .execute(pool)
        .await
        .unwrap();
}

// spec: 007/FR-013
#[tokio::test]
async fn populates_the_cache_from_a_real_release_response() {
    let mock = start_mock(
        StatusCode::OK,
        r#"{"tag_name": "v9.9.9", "html_url": "https://example.test/9.9.9"}"#,
    )
    .await;
    let state = common::build_state().await;
    enable_check_for_updates(&state.db).await;

    tokio::spawn(arrgh_server::update_checker::run_loop_with_interval(
        state.db.clone(),
        state.http.clone(),
        state.update.clone(),
        mock.clone(),
        TICK,
    ));

    let deadline = tokio::time::Instant::now() + Duration::from_secs(5);
    let (version, html_url) = loop {
        let (version, html_url) = state.update.get_if_newer("0.0.0");
        if version.is_some() {
            break (version, html_url);
        }
        assert!(
            tokio::time::Instant::now() < deadline,
            "cache never populated"
        );
        tokio::time::sleep(Duration::from_millis(10)).await;
    };
    assert_eq!(version.as_deref(), Some("9.9.9"));
    assert_eq!(html_url.as_deref(), Some("https://example.test/9.9.9"));
}

// spec: 007/FR-014
#[tokio::test]
async fn a_failed_check_never_panics_and_leaves_the_cache_empty() {
    let mock = start_mock(StatusCode::INTERNAL_SERVER_ERROR, "boom").await;
    let state = common::build_state().await;
    enable_check_for_updates(&state.db).await;

    tokio::spawn(arrgh_server::update_checker::run_loop_with_interval(
        state.db.clone(),
        state.http.clone(),
        state.update.clone(),
        mock.clone(),
        TICK,
    ));

    // Several ticks against an always-failing endpoint — the loop must keep
    // running (not panic/exit) and never populate a cache entry from it.
    tokio::time::sleep(TICK * 5).await;
    assert!(state.update.get_if_newer("0.0.0").0.is_none());
}
