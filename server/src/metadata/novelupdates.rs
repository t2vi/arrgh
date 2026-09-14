//! NovelUpdates metadata authority (ADR 0031) — novel designated authority.
//! Port of `Services/NovelUpdatesService.cs`. NovelUpdates is CF-protected;
//! search is proxied through plugin-host (which has CloakBrowser) rather
//! than hit directly. .NET kept an HTML-scraping fallback parser
//! (`ParseHtml`) that `SearchAsync` never actually calls — dead code, not
//! ported (ponytail: don't port unreachable code).

use serde::Deserialize;

#[derive(Clone, Debug, PartialEq)]
pub struct NovelUpdatesSeries {
    pub source_id: String,
    pub title: String,
    pub cover_url: Option<String>,
    pub status: String,
}

#[derive(Deserialize)]
struct PluginSearchResult {
    id: Option<String>,
    title: Option<String>,
    cover_url: Option<String>,
    status: Option<String>,
}

pub async fn search(
    http: &reqwest::Client,
    plugin_host_url: &str,
    q: &str,
) -> anyhow::Result<Vec<NovelUpdatesSeries>> {
    let url = format!(
        "{}/novelupdates/search?q={}",
        plugin_host_url.trim_end_matches('/'),
        urlencoding::encode(q)
    );
    let resp = http.get(&url).send().await?.error_for_status()?;
    let results: Vec<PluginSearchResult> = resp.json().await?;
    Ok(results
        .into_iter()
        .filter_map(|r| {
            let id = r.id.filter(|s| !s.is_empty())?;
            let title = r.title.filter(|s| !s.is_empty())?;
            Some(NovelUpdatesSeries {
                source_id: id,
                title,
                cover_url: r.cover_url,
                status: r.status.unwrap_or_else(|| "unknown".to_string()),
            })
        })
        .collect())
}
