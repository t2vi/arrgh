//! `server_settings` KV access + pure helpers (ADR 0033, S3 #125). Port of
//! `Api/Settings.cs`. No sqlx migrations — same deferral as `users.rs`:
//! .NET's EF migrations still own the schema until cutover (S10).

use std::collections::HashMap;

use sqlx::SqlitePool;

pub const DOWNLOAD_WORKERS: &str = "download_workers";
pub const DEFAULT_DOWNLOAD_WORKERS: i64 = 2;
/// Matches the Settings UI stepper (1–10); the API itself accepts any number.
pub const MAX_DOWNLOAD_WORKERS: i64 = 10;
/// "Sync interval (hours)" — how often the scheduler re-syncs library titles.
pub const INDEX_INTERVAL_HOURS: &str = "index_interval_hours";
pub const DEFAULT_INDEX_INTERVAL_HOURS: i64 = 6;
/// Matches the Settings UI stepper (1–24).
pub const MAX_INDEX_INTERVAL_HOURS: i64 = 24;
/// Global auto-download default; a title's own `auto_download` overrides it.
pub const AUTO_DOWNLOAD: &str = "auto_download";
pub const DEFAULT_AUTO_DOWNLOAD: bool = false;
/// "Library path" — where downloads (and the cover/meta cache) live. Read once at boot,
/// where it overrides the `DownloadDir` env var ("Restart required" in the UI).
pub const DOWNLOAD_DIR: &str = "download_dir";
/// Results shown per trending lane (each lane is fed by one source).
pub const TRENDING_PER_SOURCE: &str = "trending_per_source";
pub const DEFAULT_TRENDING_PER_SOURCE: i64 = 5;
/// Where manual/scheduled DB backups are written (spec 035). No default — no safe path can be
/// guessed, unlike `DOWNLOAD_DIR`'s env-var fallback; backups simply don't happen until an admin
/// sets this.
pub const BACKUP_DIR: &str = "backup_dir";
/// How often the scheduler takes an automatic DB backup, when `BACKUP_DIR` is set (spec 035).
pub const BACKUP_INTERVAL_HOURS: &str = "backup_interval_hours";
pub const DEFAULT_BACKUP_INTERVAL_HOURS: i64 = 24;

/// Saved `download_dir` if set and non-empty, else the env/config default.
pub async fn effective_download_dir(pool: &SqlitePool, env_default: &str) -> String {
    get(pool, DOWNLOAD_DIR)
        .await
        .ok()
        .flatten()
        .filter(|v| !v.trim().is_empty())
        .unwrap_or_else(|| env_default.to_string())
}

/// Current trending lane size (`trending_per_source`, clamped like on save).
pub async fn trending_per_source(pool: &SqlitePool) -> usize {
    let raw = get(pool, TRENDING_PER_SOURCE).await.ok().flatten();
    clamp_trending(parse_long(raw.as_deref(), DEFAULT_TRENDING_PER_SOURCE)) as usize
}

pub async fn get(pool: &SqlitePool, key: &str) -> sqlx::Result<Option<String>> {
    sqlx::query_scalar("SELECT value FROM server_settings WHERE key = ?")
        .bind(key)
        .fetch_optional(pool)
        .await
}

pub async fn get_all(pool: &SqlitePool) -> sqlx::Result<HashMap<String, String>> {
    let rows: Vec<(String, String)> = sqlx::query_as("SELECT key, value FROM server_settings")
        .fetch_all(pool)
        .await?;
    Ok(rows.into_iter().collect())
}

pub async fn set(pool: &SqlitePool, key: &str, value: &str) -> sqlx::Result<()> {
    sqlx::query("INSERT INTO server_settings (key, value) VALUES (?, ?) ON CONFLICT(key) DO UPDATE SET value = excluded.value")
        .bind(key)
        .bind(value)
        .execute(pool)
        .await?;
    Ok(())
}

// ── Pure helpers — mirror Settings.cs's internal statics ───────────────────

pub fn parse_long(value: Option<&str>, default: i64) -> i64 {
    value.and_then(|v| v.parse().ok()).unwrap_or(default)
}

pub fn parse_bool(value: Option<&str>, default: bool) -> bool {
    match value {
        None => default,
        Some(v) => v == "true",
    }
}

pub fn clamp_trending(value: i64) -> i64 {
    value.clamp(1, 50)
}

pub fn valid_reader_mode(value: &str) -> bool {
    matches!(value, "paged" | "scroll")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_long_valid_string_returns_value() {
        assert_eq!(parse_long(Some("4"), 2), 4);
    }

    #[test]
    fn parse_long_none_returns_default() {
        assert_eq!(parse_long(None, 2), 2);
    }

    #[test]
    fn parse_long_invalid_string_returns_default() {
        assert_eq!(parse_long(Some("not-a-number"), 6), 6);
    }

    #[test]
    fn parse_bool_true_string_returns_true() {
        assert!(parse_bool(Some("true"), false));
    }

    #[test]
    fn parse_bool_false_string_returns_false() {
        assert!(!parse_bool(Some("false"), true));
    }

    #[test]
    fn parse_bool_none_returns_default() {
        assert!(parse_bool(None, true));
    }

    #[test]
    fn parse_bool_other_string_returns_false() {
        assert!(!parse_bool(Some("yes"), false));
    }

    #[test]
    fn clamp_trending_below_1_clamps_to_1() {
        assert_eq!(clamp_trending(0), 1);
    }

    #[test]
    fn clamp_trending_above_50_clamps_to_50() {
        assert_eq!(clamp_trending(999), 50);
    }

    #[test]
    fn clamp_trending_within_passes_through() {
        assert_eq!(clamp_trending(25), 25);
    }

    #[test]
    fn clamp_trending_boundary_1_valid() {
        assert_eq!(clamp_trending(1), 1);
    }

    #[test]
    fn clamp_trending_boundary_50_valid() {
        assert_eq!(clamp_trending(50), 50);
    }

    #[test]
    fn valid_reader_mode_paged_valid() {
        assert!(valid_reader_mode("paged"));
    }

    #[test]
    fn valid_reader_mode_scroll_valid() {
        assert!(valid_reader_mode("scroll"));
    }

    #[test]
    fn valid_reader_mode_other_invalid() {
        assert!(!valid_reader_mode("continuous"));
    }

    #[test]
    fn valid_reader_mode_empty_invalid() {
        assert!(!valid_reader_mode(""));
    }
}
