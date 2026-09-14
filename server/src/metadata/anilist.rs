//! AniList metadata authority (ADR 0031) — manhwa/manhua designated
//! authority + trending lanes. Port of `Services/AniListService.cs`.

use serde_json::Value;

pub const DEFAULT_ENDPOINT: &str = "https://graphql.anilist.co";

const QUERY: &str = r#"
query ($search: String, $isAdult: Boolean) {
  Page(perPage: 25) {
    media(search: $search, type: MANGA, isAdult: $isAdult) {
      id
      title { romaji english }
      isAdult
      format
      countryOfOrigin
      status
      description(asHtml: false)
      coverImage { large }
      startDate { year }
      staff(perPage: 5) { nodes { name { full } } }
      synonyms
    }
  }
}"#;

const TRENDING_QUERY: &str = r#"
query ($country: CountryCode, $isAdult: Boolean, $perPage: Int) {
  Page(perPage: $perPage) {
    media(type: MANGA, sort: [TRENDING_DESC], countryOfOrigin: $country, isAdult: $isAdult) {
      id
      title { romaji english }
      isAdult
      format
      countryOfOrigin
      status
      description(asHtml: false)
      coverImage { large }
      startDate { year }
      staff(perPage: 5) { nodes { name { full } } }
      synonyms
    }
  }
}"#;

const SYNONYMS_QUERY: &str = r#"
query ($id: Int) {
  Media(id: $id, type: MANGA) {
    synonyms
    title { romaji english }
  }
}"#;

#[derive(Clone, Debug, PartialEq)]
pub struct AniListSeries {
    pub source_id: String,
    pub title: String,
    pub description: Option<String>,
    pub cover_url: Option<String>,
    pub status: String,
    pub content_type: String,
    pub author: Option<String>,
    pub year: Option<i32>,
    pub is_adult: bool,
}

/// Always runs a non-adult query; when `allow_explicit`, also runs an
/// adult-only query and merges (dedup by `source_id`).
pub async fn search(
    http: &reqwest::Client,
    endpoint: &str,
    q: &str,
    allow_explicit: bool,
) -> anyhow::Result<Vec<AniListSeries>> {
    let non_adult = fetch_page(http, endpoint, q, false);
    if !allow_explicit {
        return non_adult.await;
    }
    let (non_adult, adult) = tokio::join!(non_adult, fetch_page(http, endpoint, q, true));
    let (non_adult, adult) = (non_adult?, adult?);

    let mut seen = std::collections::HashSet::new();
    let mut merged = Vec::new();
    for item in non_adult.into_iter().chain(adult) {
        if seen.insert(item.source_id.clone()) {
            merged.push(item);
        }
    }
    Ok(merged)
}

async fn fetch_page(
    http: &reqwest::Client,
    endpoint: &str,
    q: &str,
    is_adult: bool,
) -> anyhow::Result<Vec<AniListSeries>> {
    let resp = http
        .post(endpoint)
        .json(&serde_json::json!({ "query": QUERY, "variables": { "search": q, "isAdult": is_adult } }))
        .send()
        .await?
        .error_for_status()?;
    let body = resp.text().await?;
    if body.trim().is_empty() {
        anyhow::bail!("AniList returned empty response");
    }
    let json: Value = serde_json::from_str(&body)?;
    Ok(parse_response(&json))
}

pub async fn trending(
    http: &reqwest::Client,
    endpoint: &str,
    country: &str,
    is_adult: bool,
    limit: u32,
) -> Vec<AniListSeries> {
    let result: anyhow::Result<Vec<AniListSeries>> = async {
        let resp = http
            .post(endpoint)
            .json(&serde_json::json!({
                "query": TRENDING_QUERY,
                "variables": { "country": country, "isAdult": is_adult, "perPage": limit },
            }))
            .send()
            .await?
            .error_for_status()?;
        let body = resp.text().await?;
        if body.trim().is_empty() {
            return Ok(Vec::new());
        }
        let json: Value = serde_json::from_str(&body)?;
        Ok(parse_response(&json))
    }
    .await;
    result.unwrap_or_default()
}

pub async fn get_synonyms(http: &reqwest::Client, endpoint: &str, media_id: &str) -> Vec<String> {
    let Ok(id) = media_id.parse::<i64>() else {
        return Vec::new();
    };
    let result: anyhow::Result<Vec<String>> = async {
        let resp = http
            .post(endpoint)
            .json(&serde_json::json!({ "query": SYNONYMS_QUERY, "variables": { "id": id } }))
            .send()
            .await?;
        if !resp.status().is_success() {
            return Ok(Vec::new());
        }
        let body = resp.text().await?;
        let json: Value = serde_json::from_str(&body)?;
        let media = json.get("data").and_then(|d| d.get("Media"));
        let Some(media) = media.filter(|m| !m.is_null()) else {
            return Ok(Vec::new());
        };

        let mut out = Vec::new();
        if let Some(syns) = media.get("synonyms").and_then(Value::as_array) {
            for s in syns {
                if let Some(s) = s.as_str() {
                    let s = s.trim();
                    if !s.is_empty() {
                        out.push(s.to_string());
                    }
                }
            }
        }
        if let Some(title) = media.get("title") {
            if let Some(en) = title
                .get("english")
                .and_then(Value::as_str)
                .filter(|s| !s.trim().is_empty())
            {
                out.push(en.to_string());
            }
            if let Some(ro) = title
                .get("romaji")
                .and_then(Value::as_str)
                .filter(|s| !s.trim().is_empty())
            {
                out.push(ro.to_string());
            }
        }
        let mut seen = std::collections::HashSet::new();
        Ok(out.into_iter().filter(|s| seen.insert(s.clone())).collect())
    }
    .await;
    result.unwrap_or_default()
}

// ── parsing ──────────────────────────────────────────────────────────────

fn parse_response(root: &Value) -> Vec<AniListSeries> {
    root.get("data")
        .and_then(|d| d.get("Page"))
        .and_then(|p| p.get("media"))
        .and_then(Value::as_array)
        .map(|arr| arr.iter().filter_map(map_entry).collect())
        .unwrap_or_default()
}

pub(crate) fn map_entry(item: &Value) -> Option<AniListSeries> {
    let id = item.get("id").and_then(Value::as_i64)?.to_string();

    let format = item.get("format").and_then(Value::as_str);
    let country = item.get("countryOfOrigin").and_then(Value::as_str);
    let content_type = map_content_type(format, country);
    if content_type == "other" {
        return None;
    }

    let title_obj = item.get("title");
    let title = title_obj
        .and_then(|t| t.get("english"))
        .and_then(Value::as_str)
        .filter(|s| !s.is_empty())
        .or_else(|| {
            title_obj
                .and_then(|t| t.get("romaji"))
                .and_then(Value::as_str)
        })
        .unwrap_or("")
        .to_string();
    if title.is_empty() {
        return None;
    }

    let description = item
        .get("description")
        .and_then(Value::as_str)
        .filter(|s| !s.is_empty())
        .map(String::from);

    let cover_url = item
        .get("coverImage")
        .and_then(|c| c.get("large"))
        .and_then(Value::as_str)
        .map(String::from);

    let status = map_status(item.get("status").and_then(Value::as_str));

    let year = item
        .get("startDate")
        .and_then(|d| d.get("year"))
        .and_then(Value::as_i64)
        .map(|y| y as i32);

    let author = item
        .get("staff")
        .and_then(|s| s.get("nodes"))
        .and_then(Value::as_array)
        .and_then(|arr| arr.first())
        .and_then(|n| n.get("name"))
        .and_then(|n| n.get("full"))
        .and_then(Value::as_str)
        .map(String::from);

    let is_adult = item
        .get("isAdult")
        .and_then(Value::as_bool)
        .unwrap_or(false);

    Some(AniListSeries {
        source_id: id,
        title,
        description,
        cover_url,
        status,
        content_type,
        author,
        year,
        is_adult,
    })
}

pub(crate) fn map_content_type(format: Option<&str>, country: Option<&str>) -> String {
    match format.map(|f| f.to_uppercase()).as_deref() {
        Some("MANHWA") => "manhwa",
        Some("MANHUA") => "manhua",
        Some("MANGA") => match country {
            Some("KR") => "manhwa",
            Some("CN") | Some("TW") => "manhua",
            _ => "manga",
        },
        Some("ONE_SHOT") => "manga",
        Some("NOVEL") | Some("LIGHT_NOVEL") => "novel",
        _ => "other",
    }
    .to_string()
}

fn map_status(s: Option<&str>) -> String {
    match s.map(|s| s.to_uppercase()).as_deref() {
        Some("FINISHED") => "complete",
        Some("RELEASING") => "ongoing",
        Some("NOT_YET_RELEASED") => "upcoming",
        Some("CANCELLED") => "cancelled",
        _ => "unknown",
    }
    .to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn map_content_type_variants() {
        assert_eq!(map_content_type(Some("MANHWA"), None), "manhwa");
        assert_eq!(map_content_type(Some("MANHUA"), None), "manhua");
        assert_eq!(map_content_type(Some("MANGA"), Some("KR")), "manhwa");
        assert_eq!(map_content_type(Some("MANGA"), Some("CN")), "manhua");
        assert_eq!(map_content_type(Some("MANGA"), Some("JP")), "manga");
        assert_eq!(map_content_type(Some("ONE_SHOT"), None), "manga");
        assert_eq!(map_content_type(Some("NOVEL"), None), "novel");
        assert_eq!(map_content_type(Some("OTHER"), None), "other");
    }

    #[test]
    fn map_entry_skips_other_format() {
        let item = serde_json::json!({
            "id": 1, "format": "NOVEL", "countryOfOrigin": "JP",
            "title": { "english": "X" },
        });
        // NOVEL maps to "novel", not "other" — should not be skipped
        assert!(map_entry(&item).is_some());

        let item2 = serde_json::json!({
            "id": 2, "format": "MUSIC", "title": { "english": "X" },
        });
        assert!(map_entry(&item2).is_none());
    }

    #[test]
    fn map_entry_prefers_english_title() {
        let item = serde_json::json!({
            "id": 5, "format": "MANHWA",
            "title": { "english": "English Name", "romaji": "Romaji Name" },
        });
        assert_eq!(map_entry(&item).unwrap().title, "English Name");
    }
}
