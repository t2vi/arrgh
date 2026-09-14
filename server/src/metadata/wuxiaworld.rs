//! WuxiaWorld metadata authority (ADR 0031) — parallel novel authority,
//! deduped against NovelUpdates (NU wins). Port of
//! `Services/WuxiaWorldMetaService.cs`. Distinct from the WuxiaWorld
//! *plugin* (chapter source, via plugin-host).

use serde_json::Value;

#[derive(Clone, Debug, PartialEq)]
pub struct WuxiaWorldSeries {
    pub source_id: String,
    pub title: String,
    pub cover_url: Option<String>,
    pub status: String,
    pub author: Option<String>,
}

pub async fn search(
    http: &reqwest::Client,
    base: &str,
    q: &str,
) -> anyhow::Result<Vec<WuxiaWorldSeries>> {
    let url = format!(
        "{base}/api/novels/search?query={}&pageSize=20",
        urlencoding::encode(q)
    );
    let resp = http
        .get(&url)
        .header("User-Agent", "Mozilla/5.0")
        .header("Accept", "application/json")
        .send()
        .await?
        .error_for_status()?;
    let json: Value = resp.json().await?;
    Ok(parse_response(&json))
}

fn parse_response(root: &Value) -> Vec<WuxiaWorldSeries> {
    root.get("items")
        .and_then(Value::as_array)
        .map(|arr| arr.iter().filter_map(map_entry).collect())
        .unwrap_or_default()
}

fn map_entry(item: &Value) -> Option<WuxiaWorldSeries> {
    let source_id = item
        .get("slug")
        .and_then(Value::as_str)
        .filter(|s| !s.is_empty())
        .map(String::from)
        .or_else(|| {
            item.get("id")
                .and_then(Value::as_i64)
                .map(|n| n.to_string())
        })?;
    let title = item
        .get("name")
        .and_then(Value::as_str)
        .filter(|s| !s.is_empty())?
        .to_string();

    let cover_url = item
        .get("coverUrl")
        .and_then(Value::as_str)
        .map(String::from);
    let status_num = item.get("status").and_then(Value::as_i64);
    let tags: Option<Vec<String>> = item.get("tags").and_then(Value::as_array).map(|arr| {
        arr.iter()
            .filter_map(|t| t.as_str().map(String::from))
            .collect()
    });

    Some(WuxiaWorldSeries {
        source_id,
        title,
        cover_url,
        status: map_status(status_num, tags.as_deref()),
        author: None, // not in search response, matches .NET
    })
}

fn map_status(status: Option<i64>, tags: Option<&[String]>) -> String {
    if let Some(tags) = tags {
        if tags.iter().any(|t| t.eq_ignore_ascii_case("completed")) {
            return "complete".into();
        }
        if tags.iter().any(|t| t.eq_ignore_ascii_case("ongoing")) {
            return "ongoing".into();
        }
    }
    match status {
        Some(0) => "complete",
        Some(1) => "ongoing",
        _ => "unknown",
    }
    .to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn map_status_prefers_tags_over_status_code() {
        assert_eq!(
            map_status(Some(1), Some(&["Completed".to_string()])),
            "complete"
        );
        assert_eq!(
            map_status(Some(0), Some(&["Ongoing".to_string()])),
            "ongoing"
        );
        assert_eq!(map_status(Some(0), None), "complete");
        assert_eq!(map_status(Some(1), None), "ongoing");
        assert_eq!(map_status(None, None), "unknown");
    }

    #[test]
    fn map_entry_falls_back_to_id_when_no_slug() {
        let item = serde_json::json!({ "id": 42, "name": "Title" });
        let s = map_entry(&item).unwrap();
        assert_eq!(s.source_id, "42");
    }

    #[test]
    fn map_entry_skips_missing_name() {
        let item = serde_json::json!({ "slug": "x" });
        assert!(map_entry(&item).is_none());
    }
}
