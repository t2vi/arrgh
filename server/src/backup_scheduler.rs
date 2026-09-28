//! Scheduled DB backups (spec 035, issue #251). Every `backup_interval_hours` ("Backup interval"
//! in Settings, re-read each cycle) takes a fresh `VACUUM INTO` snapshot into `backup_dir` — a
//! no-op, not an error, while `backup_dir` is unset (`backup::create_backup`'s caller-side
//! guard, mirrored here rather than in `backup.rs` itself). Structural mirror of
//! `scheduler.rs`'s `interval()`/`tick()`/`run_loop()` shape.

use std::path::PathBuf;
use std::time::Duration;

use sqlx::SqlitePool;

use crate::{backup, settings};

pub async fn run_loop(pool: SqlitePool) {
    loop {
        tokio::time::sleep(interval(&pool).await).await;
        tick(&pool).await;
    }
}

/// Current backup interval — no clamp range like the sync interval's 1-24h UI stepper (no UI
/// range decided for this setting yet), just the configured value or the default.
pub async fn interval(pool: &SqlitePool) -> Duration {
    let raw = settings::get(pool, settings::BACKUP_INTERVAL_HOURS)
        .await
        .ok()
        .flatten();
    let hours = settings::parse_long(raw.as_deref(), settings::DEFAULT_BACKUP_INTERVAL_HOURS);
    Duration::from_secs(hours.max(1) as u64 * 3600)
}

/// One scheduler pass. A no-op when `backup_dir` is unset — nothing to fail loudly about, same
/// as the manual "Backup now" button's `422` has no scheduled-context equivalent to surface to.
pub async fn tick(pool: &SqlitePool) {
    let dir = match settings::get(pool, settings::BACKUP_DIR).await {
        Ok(Some(v)) if !v.trim().is_empty() => PathBuf::from(v.trim().to_string()),
        Ok(_) => return,
        Err(e) => {
            tracing::warn!(error = ?e, "scheduled backup: reading backup_dir failed");
            return;
        }
    };
    if let Err(e) = backup::create_backup(pool, &dir).await {
        tracing::warn!(error = ?e, "scheduled backup failed");
    }
}
