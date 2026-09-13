//! `server_settings` KV access + pure helpers (ADR 0033, S3 #125). Port of
//! `Api/Settings.cs`. No sqlx migrations — same deferral as `users.rs`:
//! .NET's EF migrations still own the schema until cutover (S10).

use std::collections::HashMap;

use sqlx::SqlitePool;

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
