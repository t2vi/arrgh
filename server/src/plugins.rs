//! Plugin catalog (`plugin-index/index.json`) fetch, plus the pure rules for
//! in-app plugin updates (spec 031 phase A). The catalog is read live
//! (`PluginIndexUrl`, `main`'s raw URL by default) and falls back to the copy
//! baked into the image (`PluginIndexFallbackUrl`).

use std::cmp::Ordering;
use std::time::Duration;

use serde::{Deserialize, Serialize};

/// Which catalog `fetch_catalog` answered from.
pub const CATALOG_LIVE: &str = "live";
pub const CATALOG_FALLBACK: &str = "fallback";

/// plugin-host's `origin` values.
pub const ORIGIN_BUNDLED: &str = "bundled";
pub const ORIGIN_DOWNLOADED: &str = "downloaded";

/// Upper bound on one catalog fetch — the live one is on GitHub.
const CATALOG_TIMEOUT: Duration = Duration::from_secs(10);

fn default_protocol() -> u32 {
    1
}

#[derive(Deserialize, Serialize, Clone)]
pub struct PluginIndexEntry {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub description: Option<String>,
    pub version: String,
    #[serde(default)]
    pub download_url: Option<String>,
    /// sha256 (hex) of the file at `download_url`; required to install/update.
    #[serde(default)]
    pub sha256: Option<String>,
    /// Plugin protocol the bundle needs; plugin-host refuses newer ones.
    #[serde(default = "default_protocol")]
    pub protocol: u32,
    #[serde(default)]
    pub bundled: Option<bool>,
    #[serde(default)]
    pub default_explicit: bool,
    #[serde(default)]
    pub content_types: Vec<String>,
}

/// `None` on any failure (bad `file://` path, unreachable URL, bad JSON).
pub async fn fetch_index(url: &str, http: &reqwest::Client) -> Option<Vec<PluginIndexEntry>> {
    let text = if let Some(path) = url.strip_prefix("file://") {
        tokio::fs::read_to_string(path).await.ok()?
    } else {
        let res = http.get(url).timeout(CATALOG_TIMEOUT).send().await.ok()?;
        res.error_for_status().ok()?.text().await.ok()?
    };
    serde_json::from_str(&text).ok()
}

/// Live catalog, else the image copy; `None` when neither can be read (FR-002).
pub async fn fetch_catalog(
    live: &str,
    fallback: &str,
    http: &reqwest::Client,
) -> Option<(Vec<PluginIndexEntry>, &'static str)> {
    if let Some(entries) = fetch_index(live, http).await {
        return Some((entries, CATALOG_LIVE));
    }
    tracing::warn!(
        live,
        "plugin catalog unreachable — using the copy shipped with the app"
    );
    fetch_index(fallback, http)
        .await
        .map(|e| (e, CATALOG_FALLBACK))
}

/// Numeric dot-separated compare; a pre-release suffix is ignored.
/// ponytail: not full semver — add a crate if plugins ever ship pre-releases.
pub fn compare_versions(a: &str, b: &str) -> Ordering {
    let parts = |v: &str| -> Vec<u64> {
        v.split('-')
            .next()
            .unwrap_or("")
            .split('.')
            .map(|n| n.parse().unwrap_or(0))
            .collect()
    };
    let (pa, pb) = (parts(a), parts(b));
    (0..pa.len().max(pb.len()))
        .map(|i| pa.get(i).unwrap_or(&0).cmp(pb.get(i).unwrap_or(&0)))
        .find(|o| o.is_ne())
        .unwrap_or(Ordering::Equal)
}

/// Catalog newer than what's loaded; an unknown loaded version counts as older (US3 AS2).
pub fn update_available(loaded: Option<&str>, catalog: &str) -> bool {
    loaded.is_none_or(|v| compare_versions(catalog, v).is_gt())
}

/// Why this entry can't be installed/updated right now, or `None` if it can
/// (FR-005, FR-006). `host_protocol` is `None` when plugin-host didn't answer.
pub fn blocked_reason(entry: &PluginIndexEntry, host_protocol: Option<u32>) -> Option<String> {
    if entry
        .download_url
        .as_deref()
        .is_none_or(|u| u.trim().is_empty())
    {
        return Some("no download for this version in the catalog yet".into());
    }
    if entry.sha256.as_deref().is_none_or(|s| s.trim().is_empty()) {
        return Some("the catalog entry has no checksum".into());
    }
    match host_protocol {
        None => Some("plugin host unreachable".into()),
        Some(host) if entry.protocol > host => Some(format!(
            "needs a newer *ARRgh (plugin protocol {}, this one supports {host})",
            entry.protocol
        )),
        Some(_) => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn fetch_index_reads_file_url() {
        let path =
            std::env::temp_dir().join(format!("arrgh-plugins-test-{}.json", uuid::Uuid::new_v4()));
        std::fs::write(
            &path,
            r#"[{"id":"mangadex","name":"MangaDex","version":"1.0.0","content_types":["manga"]}]"#,
        )
        .unwrap();

        let entries = fetch_index(
            &format!("file://{}", path.display()),
            &reqwest::Client::new(),
        )
        .await
        .unwrap();
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].id, "mangadex");

        std::fs::remove_file(&path).unwrap();
    }

    #[tokio::test]
    async fn fetch_index_missing_file_returns_none() {
        let entries = fetch_index("file:///nonexistent/path.json", &reqwest::Client::new()).await;
        assert!(entries.is_none());
    }

    fn entry(json: &str) -> PluginIndexEntry {
        serde_json::from_str(json).unwrap()
    }

    // spec: 031/FR-001
    #[test]
    fn entry_parses_sha256_and_protocol_defaulting_to_1() {
        let e = entry(r#"{"id":"a","name":"A","version":"1.0.0","sha256":"ff"}"#);
        assert_eq!(e.sha256.as_deref(), Some("ff"));
        assert_eq!(e.protocol, 1);
        assert_eq!(
            entry(r#"{"id":"a","name":"A","version":"1.0.0","protocol":3}"#).protocol,
            3
        );
    }

    // spec: 031/FR-002
    #[tokio::test]
    async fn fetch_catalog_prefers_live_then_fallback() {
        let path = std::env::temp_dir().join(format!("arrgh-cat-{}.json", uuid::Uuid::new_v4()));
        std::fs::write(&path, r#"[{"id":"a","name":"A","version":"1.0.0"}]"#).unwrap();
        let file = format!("file://{}", path.display());
        let http = reqwest::Client::new();

        let (_, from) = fetch_catalog(&file, "file:///nope.json", &http)
            .await
            .unwrap();
        assert_eq!(from, CATALOG_LIVE);
        let (e, from) = fetch_catalog("file:///nope.json", &file, &http)
            .await
            .unwrap();
        assert_eq!((e.len(), from), (1, CATALOG_FALLBACK));
        assert!(
            fetch_catalog("file:///nope.json", "file:///nope.json", &http)
                .await
                .is_none()
        );
        std::fs::remove_file(&path).unwrap();
    }

    // spec: 031/FR-003, 031/FR-010
    #[test]
    fn versions_compare_numerically_and_unknown_is_updatable() {
        assert!(compare_versions("1.10.0", "1.9.0").is_gt());
        assert!(compare_versions("1.0", "1.0.0").is_eq());
        assert!(update_available(Some("1.0.0"), "1.0.1"));
        assert!(!update_available(Some("1.1.0"), "1.0.1"));
        assert!(update_available(None, "1.0.0"));
    }

    // spec: 031/FR-005, 031/FR-006
    #[test]
    fn blocked_reason_covers_download_checksum_and_protocol() {
        let ok = entry(
            r#"{"id":"a","name":"A","version":"1","download_url":"https://x/a.js","sha256":"ff"}"#,
        );
        assert_eq!(blocked_reason(&ok, Some(1)), None);
        assert!(blocked_reason(&ok, None).unwrap().contains("unreachable"));
        let future = PluginIndexEntry {
            protocol: 2,
            ..ok.clone()
        };
        assert!(blocked_reason(&future, Some(1))
            .unwrap()
            .contains("newer *ARRgh"));
        let nosha = PluginIndexEntry {
            sha256: None,
            ..ok.clone()
        };
        assert!(blocked_reason(&nosha, Some(1))
            .unwrap()
            .contains("checksum"));
        let nourl = PluginIndexEntry {
            download_url: Some(" ".into()),
            ..ok
        };
        assert!(blocked_reason(&nourl, Some(1))
            .unwrap()
            .contains("download"));
    }
}
