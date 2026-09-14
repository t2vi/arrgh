//! MangaUpdates metadata authority (ADR 0015, ADR 0031). Port of
//! `Services/MangaUpdatesService.cs`. Manga-authority — Discover filters its
//! results to `manga`/`one-shot` before dedup (`crate::discover::filter_mu_scope`).

use serde_json::Value;

/// Production default — overridable via `Config::mangaupdates_url` in tests
/// (a plain `reqwest::Client` can't intercept requests by host like .NET's
/// fake `HttpMessageHandler` did, so the base URL itself is the test seam).
pub const DEFAULT_BASE: &str = "https://api.mangaupdates.com/v1";

#[derive(Clone, Debug, PartialEq)]
pub struct MuSeries {
    pub series_id: u64,
    pub title: String,
    pub description: Option<String>,
    pub cover_url: Option<String>,
    pub status: String,
    pub content_type: String,
    pub author: Option<String>,
    pub year: Option<i32>,
    pub tags: Option<String>,
    pub associated_names: Vec<String>,
}

pub async fn search(http: &reqwest::Client, base: &str, q: &str) -> anyhow::Result<Vec<MuSeries>> {
    let resp = http
        .post(format!("{base}/series/search"))
        .json(&serde_json::json!({ "search": q, "stype": "title", "page": 1, "per_page": 25 }))
        .send()
        .await?
        .error_for_status()?;
    let json: Value = resp.json().await?;
    Ok(parse_search_response(&json))
}

pub async fn series_detail(
    http: &reqwest::Client,
    base: &str,
    id: u64,
) -> anyhow::Result<Option<MuSeries>> {
    let resp = http.get(format!("{base}/series/{id}")).send().await?;
    if resp.status() == reqwest::StatusCode::NOT_FOUND {
        return Ok(None);
    }
    let json: Value = resp.error_for_status()?.json().await?;
    Ok(Some(map_series(&json)))
}

/// Fetches the 20 most-recent distinct series from `releases/search`, then
/// fans out `series_detail` for each. Mirrors .NET's two-pass approach.
pub async fn latest_releases(http: &reqwest::Client, base: &str) -> anyhow::Result<Vec<MuSeries>> {
    let resp = http
        .post(format!("{base}/releases/search"))
        .json(&serde_json::json!({ "per_page": 100, "include_metadata": true }))
        .send()
        .await?
        .error_for_status()?;
    let json: Value = resp.json().await?;

    let mut seen = std::collections::HashSet::new();
    let mut series_ids = Vec::new();
    if let Some(results) = json.get("results").and_then(Value::as_array) {
        for hit in results {
            let Some(id) = hit
                .get("metadata")
                .and_then(|m| m.get("series"))
                .and_then(|s| s.get("series_id"))
                .and_then(parse_flex_u64)
            else {
                continue;
            };
            if seen.insert(id) {
                series_ids.push(id);
                if series_ids.len() >= 20 {
                    break;
                }
            }
        }
    }

    let futures = series_ids.iter().map(|&id| series_detail(http, base, id));
    let results = futures::future::join_all(futures).await;
    Ok(results
        .into_iter()
        .filter_map(|r| r.ok().flatten())
        .collect())
}

// ── parsing ──────────────────────────────────────────────────────────────

fn parse_search_response(root: &Value) -> Vec<MuSeries> {
    root.get("results")
        .and_then(Value::as_array)
        .map(|arr| {
            arr.iter()
                .filter_map(|hit| hit.get("record"))
                .map(map_series)
                .collect()
        })
        .unwrap_or_default()
}

pub(crate) fn map_series(rec: &Value) -> MuSeries {
    let series_id = rec.get("series_id").and_then(parse_flex_u64).unwrap_or(0);
    let title = rec
        .get("title")
        .and_then(Value::as_str)
        .unwrap_or("")
        .to_string();

    let description = rec
        .get("description")
        .and_then(Value::as_str)
        .map(strip_html)
        .filter(|s| !s.is_empty());

    let cover_url = rec
        .get("image")
        .and_then(|i| i.get("url"))
        .and_then(|u| u.get("original"))
        .and_then(Value::as_str)
        .filter(|s| !s.is_empty())
        .map(String::from);

    let year = rec.get("year").and_then(|y| match y {
        Value::String(s) => s.parse().ok(),
        Value::Number(_) => y.as_i64().map(|n| n as i32),
        _ => None,
    });

    let content_type = rec
        .get("type")
        .and_then(Value::as_str)
        .map(map_content_type)
        .unwrap_or_else(|| "manga".to_string());

    let status = rec
        .get("status")
        .and_then(Value::as_str)
        .map(|s| s.to_lowercase())
        .unwrap_or_else(|| "unknown".to_string());

    let author = rec
        .get("authors")
        .and_then(Value::as_array)
        .and_then(|authors| {
            let preferred = authors
                .iter()
                .find(|a| a.get("type").and_then(Value::as_str) == Some("Author"));
            preferred
                .or_else(|| authors.first())
                .and_then(|a| a.get("name"))
                .and_then(Value::as_str)
                .map(String::from)
        });

    let tags = rec
        .get("genres")
        .and_then(Value::as_array)
        .and_then(|genres| {
            let list: Vec<String> = genres
                .iter()
                .filter_map(|g| g.get("genre").and_then(Value::as_str))
                .map(|genre| {
                    let lower = genre.to_lowercase();
                    if lower == "hentai" {
                        "hentai".to_string()
                    } else if ["adult", "smut", "18+", "erotic"].contains(&lower.as_str()) {
                        "adult".to_string()
                    } else {
                        genre.to_string()
                    }
                })
                .collect();
            (!list.is_empty()).then(|| list.join(","))
        });

    let associated_names = rec
        .get("associated")
        .and_then(Value::as_array)
        .map(|arr| {
            arr.iter()
                .filter_map(|a| a.get("title").and_then(Value::as_str))
                .map(String::from)
                .collect()
        })
        .unwrap_or_default();

    MuSeries {
        series_id,
        title,
        description,
        cover_url,
        status,
        content_type,
        author,
        year,
        tags,
        associated_names,
    }
}

pub(crate) fn map_content_type(t: &str) -> String {
    match t.to_lowercase().as_str() {
        "manhwa" => "manhwa",
        "manhua" => "manhua",
        "novel" | "web novel" | "light novel" | "oel" => "novel",
        _ => "manga",
    }
    .to_string()
}

pub(crate) fn strip_html(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut in_tag = false;
    for c in s.chars() {
        match c {
            '<' => in_tag = true,
            '>' => in_tag = false,
            _ if !in_tag => out.push(c),
            _ => {}
        }
    }
    out.trim().to_string()
}

fn parse_flex_u64(v: &Value) -> Option<u64> {
    match v {
        Value::Number(n) => n.as_u64(),
        Value::String(s) => s.parse().ok(),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn map_content_type_variants() {
        assert_eq!(map_content_type("Manhwa"), "manhwa");
        assert_eq!(map_content_type("Manhua"), "manhua");
        assert_eq!(map_content_type("Novel"), "novel");
        assert_eq!(map_content_type("Light Novel"), "novel");
        assert_eq!(map_content_type("Manga"), "manga");
        assert_eq!(map_content_type("weird"), "manga");
    }

    #[test]
    fn strip_html_removes_tags() {
        assert_eq!(strip_html("<p>Hello <b>world</b></p>"), "Hello world");
    }

    #[test]
    fn map_series_parses_full_record() {
        let rec = serde_json::json!({
            "series_id": "12345",
            "title": "Test Series",
            "description": "<p>Desc</p>",
            "image": { "url": { "original": "http://x/cover.jpg" } },
            "year": "2020",
            "type": "Manhwa",
            "status": "Complete",
            "authors": [{ "type": "Author", "name": "Someone" }],
            "genres": [{ "genre": "Hentai" }, { "genre": "Action" }],
            "associated": [{ "title": "Alt Name" }],
        });
        let s = map_series(&rec);
        assert_eq!(s.series_id, 12345);
        assert_eq!(s.title, "Test Series");
        assert_eq!(s.description.as_deref(), Some("Desc"));
        assert_eq!(s.cover_url.as_deref(), Some("http://x/cover.jpg"));
        assert_eq!(s.year, Some(2020));
        assert_eq!(s.content_type, "manhwa");
        assert_eq!(s.status, "complete");
        assert_eq!(s.author.as_deref(), Some("Someone"));
        assert_eq!(s.tags.as_deref(), Some("hentai,Action"));
        assert_eq!(s.associated_names, vec!["Alt Name".to_string()]);
    }
}
