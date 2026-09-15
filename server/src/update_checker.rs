//! Background GitHub-release poller (ADR 0033, S10 #132) — port of
//! `Services/UpdateCheckerService.cs`. Not ported at S0 with the rest of
//! `UpdateCache`: `GET /api/version` only needed the cache to *exist* (and
//! correctly report "no update" empty), not be populated, until S10 made
//! .NET's poller disappear for good.
//!
//! Polls `https://api.github.com/repos/t2vi/arrgh/releases/latest` hourly
//! when the `check_for_updates` server setting is `"true"`; clears the
//! cache otherwise (covers the user flipping the setting off).

use std::sync::Arc;
use std::time::Duration;

use sqlx::SqlitePool;

use crate::state::UpdateCache;

const REPO: &str = "t2vi/arrgh";
const INTERVAL: Duration = Duration::from_secs(3600);

pub async fn run_loop(pool: SqlitePool, http: reqwest::Client, cache: Arc<UpdateCache>) {
    run_loop_with_interval(pool, http, cache, INTERVAL).await
}

/// Test seam — a shorter interval keeps integration tests fast.
pub async fn run_loop_with_interval(
    pool: SqlitePool,
    http: reqwest::Client,
    cache: Arc<UpdateCache>,
    interval: Duration,
) {
    loop {
        if let Err(e) = check(&pool, &http, &cache).await {
            tracing::debug!(error = ?e, "update check failed");
        }
        tokio::time::sleep(interval).await;
    }
}

async fn check(
    pool: &SqlitePool,
    http: &reqwest::Client,
    cache: &UpdateCache,
) -> anyhow::Result<()> {
    let enabled: Option<String> =
        sqlx::query_scalar("SELECT value FROM server_settings WHERE key = 'check_for_updates'")
            .fetch_optional(pool)
            .await?;
    if enabled.as_deref() != Some("true") {
        cache.clear();
        return Ok(());
    }

    let url = format!("https://api.github.com/repos/{REPO}/releases/latest");
    let res = http
        .get(&url)
        .header("User-Agent", "arrgh-server")
        .header("Accept", "application/vnd.github+json")
        .send()
        .await?;
    if !res.status().is_success() {
        return Ok(());
    }

    let json: serde_json::Value = res.json().await?;
    if let Some((version, html_url)) = parse_release(&json) {
        cache.set(version, html_url);
    }
    Ok(())
}

/// Pure — extracted for testability. `None` on a malformed/incomplete
/// response (caller just skips updating the cache, matching .NET's
/// `if (tag is null || htmlUrl is null) return;`).
fn parse_release(json: &serde_json::Value) -> Option<(String, String)> {
    let tag = json.get("tag_name")?.as_str()?;
    let html_url = json.get("html_url")?.as_str()?.to_string();
    Some((tag.trim_start_matches('v').to_string(), html_url))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_release_strips_v_prefix() {
        let json =
            serde_json::json!({"tag_name": "v1.2.3", "html_url": "https://example.com/1.2.3"});
        assert_eq!(
            parse_release(&json),
            Some(("1.2.3".to_string(), "https://example.com/1.2.3".to_string()))
        );
    }

    #[test]
    fn parse_release_without_v_prefix() {
        let json = serde_json::json!({"tag_name": "1.2.3", "html_url": "https://example.com"});
        assert_eq!(parse_release(&json).unwrap().0, "1.2.3");
    }

    #[test]
    fn parse_release_missing_tag_returns_none() {
        let json = serde_json::json!({"html_url": "https://example.com"});
        assert_eq!(parse_release(&json), None);
    }

    #[test]
    fn parse_release_missing_html_url_returns_none() {
        let json = serde_json::json!({"tag_name": "v1.0.0"});
        assert_eq!(parse_release(&json), None);
    }
}
