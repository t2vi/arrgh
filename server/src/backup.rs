//! DB backup + restore (spec 035, issue #251). `VACUUM INTO` for backups (atomic, safe under
//! live writes — SQLite's own consistent-snapshot mechanism, core since 3.27). Restore never
//! hot-swaps the live `SqlitePool` (`AppState.db` is read directly by hundreds of call sites) —
//! it validates a candidate file, atomically renames it over `DatabasePath`, then exits the
//! process; `docker-compose.yml`'s `restart: unless-stopped` brings it back up, and
//! `connect_db`'s unconditional `sqlx::migrate!` on every boot brings the restored file up to
//! schema for free. See `specs/035-db-backup-restore/research.md`.

use std::path::Path;

use sqlx::sqlite::SqliteConnectOptions;
use sqlx::Connection;
use sqlx::{SqliteConnection, SqlitePool};
use time::macros::format_description;
use time::OffsetDateTime;

use crate::error::{AppError, AppResult};

const PREFIX: &str = "arrgh-backup-";
const SUFFIX: &str = ".db";

#[derive(Debug, Clone, serde::Serialize)]
pub struct BackupInfo {
    pub filename: String,
    pub created_at: String,
    pub size_bytes: u64,
}

/// `arrgh-backup-{YYYY-MM-DDTHH-MM-SS}.db` — colons replaced with dashes for filesystem safety
/// (Windows/some NAS filesystems reject `:` in filenames).
pub fn backup_filename(now: OffsetDateTime) -> String {
    let fmt = format_description!("[year]-[month]-[day]T[hour]-[minute]-[second]");
    format!(
        "{PREFIX}{}{SUFFIX}",
        now.format(&fmt).expect("static format")
    )
}

/// Only files matching the exact `arrgh-backup-*.db` shape are ever listed, deleted, or
/// restored — anything else in the configured directory is ignored.
pub fn is_backup_filename(name: &str) -> bool {
    name.starts_with(PREFIX) && name.ends_with(SUFFIX) && name.len() > PREFIX.len() + SUFFIX.len()
}

/// Lists `dir`'s backups, newest first. A missing/unset directory is `Ok(vec![])`, not an error
/// — there's simply nothing to list yet.
pub async fn list_backups(dir: &Path) -> std::io::Result<Vec<BackupInfo>> {
    let mut entries = match tokio::fs::read_dir(dir).await {
        Ok(e) => e,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(e) => return Err(e),
    };
    let mut out = Vec::new();
    while let Some(entry) = entries.next_entry().await? {
        let name = entry.file_name().to_string_lossy().into_owned();
        if !is_backup_filename(&name) {
            continue;
        }
        let meta = entry.metadata().await?;
        let created_at = meta
            .modified()
            .ok()
            .map(|t| {
                let odt: OffsetDateTime = t.into();
                let fmt = format_description!("[year]-[month]-[day]T[hour]:[minute]:[second]");
                odt.format(&fmt).unwrap_or_default()
            })
            .unwrap_or_default();
        out.push(BackupInfo {
            filename: name,
            created_at,
            size_bytes: meta.len(),
        });
    }
    out.sort_by(|a, b| b.filename.cmp(&a.filename)); // timestamp-named ⇒ lexicographic = newest first
    Ok(out)
}

/// Takes a fresh snapshot of `pool`'s database into `dir` via `VACUUM INTO`. Caller is
/// responsible for refusing an empty/unset `dir` before calling this (that's a `422`, not an
/// I/O error).
pub async fn create_backup(pool: &SqlitePool, dir: &Path) -> AppResult<BackupInfo> {
    tokio::fs::create_dir_all(dir)
        .await
        .map_err(anyhow::Error::from)?;
    let filename = backup_filename(OffsetDateTime::now_utc());
    let dest = dir.join(&filename);
    sqlx::query("VACUUM INTO ?")
        .bind(dest.to_string_lossy().as_ref())
        .execute(pool)
        .await
        .map_err(anyhow::Error::from)?;
    let meta = tokio::fs::metadata(&dest)
        .await
        .map_err(anyhow::Error::from)?;
    let created_at = {
        let fmt = format_description!("[year]-[month]-[day]T[hour]:[minute]:[second]");
        OffsetDateTime::now_utc().format(&fmt).unwrap_or_default()
    };
    Ok(BackupInfo {
        filename,
        created_at,
        size_bytes: meta.len(),
    })
}

/// Deletes `dir/{filename}`. `filename` must already be validated by [`is_backup_filename`] —
/// this function trusts that guard rather than re-checking, so callers must not skip it (path
/// traversal / arbitrary-file-delete guard).
pub async fn delete_backup(dir: &Path, filename: &str) -> AppResult<bool> {
    if !is_backup_filename(filename) {
        return Ok(false);
    }
    match tokio::fs::remove_file(dir.join(filename)).await {
        Ok(()) => Ok(true),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(false),
        Err(e) => Err(AppError::from(anyhow::Error::from(e))),
    }
}

/// The compiled-in migration set's own highest known version — the same `Migrator` `connect_db`
/// runs on every boot.
pub fn migration_max_version() -> i64 {
    sqlx::migrate!("./migrations")
        .iter()
        .map(|m| m.version)
        .max()
        .unwrap_or(0)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RestoreRejection {
    /// Not a readable SQLite database at all.
    Invalid,
    /// Its highest applied migration is newer than this binary knows about.
    TooNew,
}

/// Validates a restore candidate *without* touching the live pool — opens its own short-lived
/// connection to `path`. Rejects anything that isn't a genuine, intact SQLite file, and anything
/// whose schema is ahead of what this binary's own migrations know about (can't safely
/// downgrade). A candidate with no `_sqlx_migrations` table at all (empty / pre-migration
/// database) is treated as version 0 — always acceptable, since the restart-time
/// `sqlx::migrate!` brings it forward.
pub async fn validate_candidate(path: &Path) -> Result<(), RestoreRejection> {
    let opts = SqliteConnectOptions::new()
        .filename(path)
        .create_if_missing(false);
    let mut conn = SqliteConnection::connect_with(&opts)
        .await
        .map_err(|_| RestoreRejection::Invalid)?;

    let has_migrations_table: bool = sqlx::query_scalar(
        "SELECT EXISTS(SELECT 1 FROM sqlite_master WHERE type='table' AND name='_sqlx_migrations')",
    )
    .fetch_one(&mut conn)
    .await
    .map_err(|_| RestoreRejection::Invalid)?;

    let candidate_max: i64 = if has_migrations_table {
        sqlx::query_scalar("SELECT COALESCE(MAX(version), 0) FROM _sqlx_migrations")
            .fetch_one(&mut conn)
            .await
            .map_err(|_| RestoreRejection::Invalid)?
    } else {
        0
    };

    if candidate_max > migration_max_version() {
        return Err(RestoreRejection::TooNew);
    }
    Ok(())
}

/// Atomically replaces the live database file with `candidate` — `candidate` MUST already sit in
/// `database_path`'s own directory (`std::fs::rename` is only atomic within one filesystem).
/// Caller restarts the process after this returns `Ok`; that's a separate step
/// (`std::process::exit`) deliberately not folded in here so tests can assert the swap happened
/// without killing the test process.
pub fn swap_in(candidate: &Path, database_path: &str) -> std::io::Result<()> {
    std::fs::rename(candidate, database_path)
}

#[cfg(test)]
mod tests {
    use super::*;

    // spec: 035/FR-004
    #[test]
    fn backup_filename_has_no_colons() {
        let now = OffsetDateTime::from_unix_timestamp(1_790_000_000).unwrap();
        let name = backup_filename(now);
        assert!(name.starts_with("arrgh-backup-"));
        assert!(name.ends_with(".db"));
        assert!(!name.contains(':'));
    }

    // spec: 035/FR-006
    #[test]
    fn is_backup_filename_accepts_the_generated_shape() {
        let now = OffsetDateTime::from_unix_timestamp(1_790_000_000).unwrap();
        assert!(is_backup_filename(&backup_filename(now)));
    }

    // spec: 035/FR-006
    #[test]
    fn is_backup_filename_rejects_unrelated_files() {
        assert!(!is_backup_filename("some-other-file.db"));
        assert!(!is_backup_filename("arrgh-backup-.db"));
        assert!(!is_backup_filename("not-a-backup.txt"));
        assert!(!is_backup_filename("arrgh-backup-2026-01-01T00-00-00.txt"));
    }

    // spec: 035/FR-009
    #[test]
    fn migration_max_version_is_positive() {
        assert!(migration_max_version() > 0);
    }
}
