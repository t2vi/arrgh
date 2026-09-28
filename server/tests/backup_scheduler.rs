//! Scheduled DB backups (spec 035, issue #251).

mod common;

use arrgh_server::backup_scheduler::{interval, tick};
use std::time::Duration;

// spec: 035/FR-003
#[tokio::test]
async fn interval_defaults_and_reads_the_configured_value() {
    let state = common::build_state().await;
    assert_eq!(
        interval(&state.db).await,
        Duration::from_secs(arrgh_server::settings::DEFAULT_BACKUP_INTERVAL_HOURS as u64 * 3600)
    );

    arrgh_server::settings::set(
        &state.db,
        arrgh_server::settings::BACKUP_INTERVAL_HOURS,
        "3",
    )
    .await
    .unwrap();
    assert_eq!(interval(&state.db).await, Duration::from_secs(3 * 3600));
}

// spec: 035/FR-003
#[tokio::test]
async fn tick_with_configured_destination_produces_a_backup() {
    let state = common::build_state().await;
    let dir = std::env::temp_dir().join(format!("arrgh-backup-test-{}", uuid::Uuid::new_v4()));
    arrgh_server::settings::set(
        &state.db,
        arrgh_server::settings::BACKUP_DIR,
        dir.to_str().unwrap(),
    )
    .await
    .unwrap();

    tick(&state.db).await;

    let backups = arrgh_server::backup::list_backups(&dir).await.unwrap();
    assert_eq!(backups.len(), 1);
}

// spec: 035/FR-003 (US4 acceptance scenario 2 — "no backup is attempted... nothing to fail loudly about")
#[tokio::test]
async fn tick_with_no_destination_configured_is_a_silent_no_op() {
    let state = common::build_state().await;
    tick(&state.db).await; // must not panic
}
