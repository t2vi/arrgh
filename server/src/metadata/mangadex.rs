//! MangaDex metadata authority (ADR 0031) — manhua designated authority.
//! Port of `Services/MangaDexMetaService.cs`. Distinct from the MangaDex
//! *plugin* (chapter source, via plugin-host) — this hits `api.mangadex.org`
//! directly for search metadata only.

use serde_json::Value;

pub const DEFAULT_BASE: &str = "https://api.mangadex.org";

#[derive(Clone, Debug, PartialEq)]
pub struct MangaDexSeries {
    pub source_id: String,
    pub title: String,
    pub description: Option<String>,
    pub cover_url: Option<String>,
    pub status: String,
    pub content_type: String,
    pub author: Option<String>,
    pub year: Option<i32>,
}

pub async fn search(
    http: &reqwest::Client,
    base: &str,
    q: &str,
) -> anyhow::Result<Vec<MangaDexSeries>> {
    let url = format!(
        "{base}/manga?title={}&limit=25&contentRating[]=safe&contentRating[]=suggestive&includes[]=author&order[relevance]=desc",
        urlencoding::encode(q)
    );
    let resp = http.get(&url).send().await?.error_for_status()?;
    let body = resp.text().await?;
    if body.trim().is_empty() {
        anyhow::bail!("MangaDex returned empty response");
    }
    let json: Value = serde_json::from_str(&body)?;
    Ok(parse_response(&json))
}

fn parse_response(root: &Value) -> Vec<MangaDexSeries> {
    root.get("data")
        .and_then(Value::as_array)
        .map(|arr| arr.iter().filter_map(map_entry).collect())
        .unwrap_or_default()
}

pub(crate) fn map_entry(item: &Value) -> Option<MangaDexSeries> {
    let id = item.get("id").and_then(Value::as_str)?.to_string();
    let attrs = item.get("attributes")?;

    let orig_lang = attrs.get("originalLanguage").and_then(Value::as_str);
    let content_type = map_content_type(orig_lang);
    if content_type != "manhua" {
        return None;
    }

    let title_obj = attrs.get("title")?;
    let title = title_obj
        .get("en")
        .and_then(Value::as_str)
        .filter(|s| !s.is_empty())
        .map(String::from)
        .or_else(|| {
            title_obj.as_object().and_then(|o| {
                o.values()
                    .find_map(|v| v.as_str().filter(|s| !s.is_empty()))
                    .map(String::from)
            })
        })?;

    let description = attrs
        .get("description")
        .and_then(|d| d.get("en"))
        .and_then(Value::as_str)
        .filter(|s| !s.is_empty())
        .map(String::from);

    let status = map_status(attrs.get("status").and_then(Value::as_str));
    let year = attrs.get("year").and_then(Value::as_i64).map(|y| y as i32);

    let author = item
        .get("relationships")
        .and_then(Value::as_array)
        .and_then(|rels| {
            rels.iter()
                .find(|r| r.get("type").and_then(Value::as_str) == Some("author"))
                .and_then(|r| r.get("attributes"))
                .and_then(|a| a.get("name"))
                .and_then(Value::as_str)
                .map(String::from)
        });

    Some(MangaDexSeries {
        source_id: id,
        title,
        description,
        cover_url: None, // not resolved from search response, matches .NET
        status,
        content_type,
        author,
        year,
    })
}

pub(crate) fn map_content_type(orig_lang: Option<&str>) -> String {
    match orig_lang {
        Some("zh") | Some("zh-hk") | Some("zh-ro") => "manhua",
        _ => "other",
    }
    .to_string()
}

fn map_status(s: Option<&str>) -> String {
    match s.map(|s| s.to_lowercase()).as_deref() {
        Some("completed") => "complete",
        Some("ongoing") => "ongoing",
        Some("cancelled") => "cancelled",
        Some("hiatus") => "hiatus",
        _ => "unknown",
    }
    .to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn map_content_type_variants() {
        assert_eq!(map_content_type(Some("zh")), "manhua");
        assert_eq!(map_content_type(Some("zh-hk")), "manhua");
        assert_eq!(map_content_type(Some("ja")), "other");
        assert_eq!(map_content_type(None), "other");
    }

    #[test]
    fn map_entry_skips_non_manhua() {
        let item = serde_json::json!({
            "id": "abc",
            "attributes": { "originalLanguage": "ja", "title": { "en": "X" } },
        });
        assert!(map_entry(&item).is_none());
    }

    #[test]
    fn map_entry_parses_manhua() {
        let item = serde_json::json!({
            "id": "abc",
            "attributes": {
                "originalLanguage": "zh",
                "title": { "en": "Chinese Comic" },
                "description": { "en": "A description" },
                "status": "ongoing",
                "year": 2021,
            },
            "relationships": [
                { "type": "author", "attributes": { "name": "Author Name" } },
            ],
        });
        let s = map_entry(&item).unwrap();
        assert_eq!(s.source_id, "abc");
        assert_eq!(s.title, "Chinese Comic");
        assert_eq!(s.content_type, "manhua");
        assert_eq!(s.description.as_deref(), Some("A description"));
        assert_eq!(s.status, "ongoing");
        assert_eq!(s.year, Some(2021));
        assert_eq!(s.author.as_deref(), Some("Author Name"));
        assert_eq!(s.cover_url, None);
    }
}
