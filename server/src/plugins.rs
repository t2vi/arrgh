//! Plugin index fetch — port of `Api/Plugins.cs`'s data half (ADR 0033,
//! S9 #131). Proxies the bundled `plugin-index.json` (a `file://` URL in
//! production, an `http(s)://` URL in tests/self-hosted overrides) and
//! plugin-host's install/uninstall endpoints.

use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize, Clone)]
pub struct PluginIndexEntry {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub description: Option<String>,
    pub version: String,
    #[serde(default)]
    pub download_url: Option<String>,
    #[serde(default)]
    pub bundled: Option<bool>,
    #[serde(default)]
    pub default_explicit: bool,
    #[serde(default)]
    pub content_types: Vec<String>,
}

/// `None` on any failure (bad `file://` path, unreachable URL, bad JSON) —
/// caller maps that to a 502, matching `FetchIndexAsync`'s catch-all.
pub async fn fetch_index(url: &str, http: &reqwest::Client) -> Option<Vec<PluginIndexEntry>> {
    let text = if let Some(path) = url.strip_prefix("file://") {
        tokio::fs::read_to_string(path).await.ok()?
    } else {
        http.get(url).send().await.ok()?.text().await.ok()?
    };
    serde_json::from_str(&text).ok()
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
}
