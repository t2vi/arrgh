use anyhow::Result;
use reqwest::Client;
use serde::{Deserialize, Deserializer};

const BASE: &str = "https://api.mangaupdates.com/v1";

fn deserialize_u64_flexible<'de, D: Deserializer<'de>>(d: D) -> std::result::Result<u64, D::Error> {
    let v = serde_json::Value::deserialize(d)?;
    match &v {
        serde_json::Value::Number(n) => n.as_u64().ok_or_else(|| serde::de::Error::custom("expected u64")),
        serde_json::Value::String(s) => s.parse::<u64>().map_err(serde::de::Error::custom),
        _ => Err(serde::de::Error::custom(format!("expected number or string, got {v}"))),
    }
}

#[derive(Clone, Debug)]
pub struct MuSeries {
    pub series_id: u64,
    pub title: String,
    pub description: Option<String>,
    pub cover_url: Option<String>,
    pub status: String,
    pub content_type: String,
    pub author: Option<String>,
    pub year: Option<i64>,
    pub tags: Option<String>,
    pub associated_names: Vec<String>,
}

// ── Wire types ────────────────────────────────────────────────────────────────

#[derive(Deserialize)]
struct SearchResponse {
    results: Vec<SearchHit>,
}

#[derive(Deserialize)]
struct SearchHit {
    record: SeriesRecord,
}

#[derive(Deserialize)]
struct SeriesRecord {
    #[serde(deserialize_with = "deserialize_u64_flexible")]
    series_id: u64,
    title: String,
    description: Option<String>,
    image: Option<SeriesImage>,
    #[serde(rename = "type")]
    series_type: Option<String>,
    year: Option<serde_json::Value>,
    status: Option<String>,
    authors: Option<Vec<MuAuthor>>,
    genres: Option<Vec<MuGenre>>,
    associated: Option<Vec<AssociatedName>>,
}

#[derive(Deserialize)]
struct AssociatedName {
    title: String,
}

#[derive(Deserialize)]
struct SeriesImage {
    url: Option<ImageUrls>,
}

#[derive(Deserialize)]
struct ImageUrls {
    original: Option<String>,
}

#[derive(Deserialize)]
struct MuAuthor {
    name: String,
    #[serde(rename = "type")]
    author_type: Option<String>,
}

#[derive(Deserialize)]
struct MuGenre {
    genre: String,
}

#[derive(Deserialize)]
struct ReleasesResponse {
    results: Vec<ReleaseHit>,
}

#[derive(Deserialize)]
struct ReleaseHit {
    metadata: Option<ReleaseMetadata>,
}

#[derive(Deserialize)]
struct ReleaseMetadata {
    // Only series_id + title available here — not a full SeriesRecord
    series: Option<ReleaseSeries>,
}

#[derive(Deserialize)]
struct ReleaseSeries {
    #[serde(deserialize_with = "deserialize_u64_flexible")]
    series_id: u64,
}

// ── Mapping ───────────────────────────────────────────────────────────────────

fn map_content_type(t: Option<&str>) -> String {
    match t.map(str::to_lowercase).as_deref() {
        Some("manhwa") => "manhwa".to_string(),
        Some("manhua") => "manhua".to_string(),
        Some("novel") | Some("web novel") | Some("light novel") | Some("oel") => "novel".to_string(),
        _ => "manga".to_string(),
    }
}

fn map_series(rec: SeriesRecord) -> MuSeries {
    let cover_url = rec.image
        .and_then(|i| i.url)
        .and_then(|u| u.original)
        .filter(|u| !u.is_empty());

    let year: Option<i64> = rec.year.and_then(|v| match v {
        serde_json::Value::String(s) => s.parse().ok(),
        serde_json::Value::Number(n) => n.as_i64(),
        _ => None,
    });

    let author = rec.authors.as_deref().and_then(|authors| {
        authors
            .iter()
            .find(|a| a.author_type.as_deref() == Some("Author"))
            .or_else(|| authors.first())
            .map(|a| a.name.clone())
    });

    let tags: Option<String> = rec
        .genres
        .map(|genres| {
            genres
                .iter()
                .map(|g| {
                    let lower = g.genre.to_lowercase();
                    if lower == "hentai" {
                        "hentai".to_string()
                    } else if ["adult", "smut", "18+", "erotic"]
                        .iter()
                        .any(|&e| lower == e)
                    {
                        "adult".to_string()
                    } else {
                        g.genre.clone()
                    }
                })
                .collect::<Vec<_>>()
                .join(",")
        })
        .filter(|s| !s.is_empty());

    let status = rec
        .status
        .unwrap_or_else(|| "unknown".to_string())
        .to_lowercase();

    // Strip basic HTML tags from description
    let description = rec.description.map(|d| strip_html(&d)).filter(|s| !s.is_empty());

    let associated_names = rec.associated
        .unwrap_or_default()
        .into_iter()
        .map(|a| a.title)
        .collect();

    MuSeries {
        series_id: rec.series_id,
        title: rec.title,
        description,
        cover_url,
        status,
        content_type: map_content_type(rec.series_type.as_deref()),
        author,
        year,
        tags,
        associated_names,
    }
}

fn strip_html(s: &str) -> String {
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

// ── Client ────────────────────────────────────────────────────────────────────

pub struct MangaUpdatesClient {
    client: Client,
}

impl MangaUpdatesClient {
    pub fn new(client: &Client) -> Self {
        Self { client: client.clone() }
    }

    pub async fn search(&self, q: &str, page: u32) -> Result<Vec<MuSeries>> {
        let body = serde_json::json!({
            "search": q,
            "stype": "title",
            "page": page,
            "per_page": 25
        });
        let resp = self
            .client
            .post(format!("{BASE}/series/search"))
            .json(&body)
            .send()
            .await?
            .error_for_status()?
            .json::<SearchResponse>()
            .await?;
        Ok(resp.results.into_iter().map(|h| map_series(h.record)).collect())
    }

    pub async fn series_detail(&self, id: u64) -> Result<Option<MuSeries>> {
        let resp = self
            .client
            .get(format!("{BASE}/series/{id}"))
            .send()
            .await?;
        if resp.status() == reqwest::StatusCode::NOT_FOUND {
            return Ok(None);
        }
        let rec = resp.error_for_status()?.json::<SeriesRecord>().await?;
        Ok(Some(map_series(rec)))
    }

    pub async fn latest_releases(&self) -> Result<Vec<MuSeries>> {
        // Step 1: get recent releases to collect unique series_ids
        let body = serde_json::json!({
            "per_page": 100,
            "include_metadata": true
        });
        let raw = self
            .client
            .post(format!("{BASE}/releases/search"))
            .json(&body)
            .send()
            .await?
            .error_for_status()?
            .text()
            .await?;
        let resp: ReleasesResponse = serde_json::from_str(&raw).map_err(|e| {
            tracing::error!("MangaUpdates releases/search decode error: {e}\nraw: {raw}");
            e
        })?;

        let mut seen = std::collections::HashSet::new();
        let series_ids: Vec<u64> = resp
            .results
            .into_iter()
            .filter_map(|h| h.metadata?.series.map(|s| s.series_id))
            .filter(|id| seen.insert(*id))
            .take(20)
            .collect();

        if series_ids.is_empty() {
            return Ok(vec![]);
        }

        // Step 2: fetch full series details in parallel (capped at 20)
        let tasks: Vec<_> = series_ids
            .into_iter()
            .map(|id| {
                let client = self.client.clone();
                tokio::spawn(async move {
                    let resp = client.get(format!("{BASE}/series/{id}")).send().await?;
                    if resp.status() == reqwest::StatusCode::NOT_FOUND {
                        return Ok::<Option<MuSeries>, anyhow::Error>(None);
                    }
                    let rec = resp.error_for_status()?.json::<SeriesRecord>().await?;
                    Ok(Some(map_series(rec)))
                })
            })
            .collect();

        let mut results = Vec::with_capacity(tasks.len());
        for task in tasks {
            match task.await {
                Ok(Ok(Some(s))) => results.push(s),
                Ok(Ok(None)) => {}
                Ok(Err(e)) => tracing::debug!("series detail fetch error: {}", e),
                Err(e) => tracing::debug!("series detail task error: {}", e),
            }
        }
        Ok(results)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ── ReleasesResponse ──────────────────────────────────────────────────────

    #[test]
    fn releases_response_numeric_series_id() {
        let json = r#"{"results":[{"metadata":{"series":{"series_id":12345}}}]}"#;
        let r: ReleasesResponse = serde_json::from_str(json).unwrap();
        assert_eq!(r.results[0].metadata.as_ref().unwrap().series.as_ref().unwrap().series_id, 12345);
    }

    #[test]
    fn releases_response_string_series_id() {
        let json = r#"{"results":[{"metadata":{"series":{"series_id":"67890"}}}]}"#;
        let r: ReleasesResponse = serde_json::from_str(json).unwrap();
        assert_eq!(r.results[0].metadata.as_ref().unwrap().series.as_ref().unwrap().series_id, 67890);
    }

    #[test]
    fn releases_response_null_metadata() {
        let json = r#"{"results":[{"metadata":null},{"metadata":{"series":{"series_id":1}}}]}"#;
        let r: ReleasesResponse = serde_json::from_str(json).unwrap();
        assert!(r.results[0].metadata.is_none());
        assert_eq!(r.results[1].metadata.as_ref().unwrap().series.as_ref().unwrap().series_id, 1);
    }

    #[test]
    fn releases_response_null_series() {
        let json = r#"{"results":[{"metadata":{"series":null}}]}"#;
        let r: ReleasesResponse = serde_json::from_str(json).unwrap();
        assert!(r.results[0].metadata.as_ref().unwrap().series.is_none());
    }

    #[test]
    fn releases_response_missing_metadata_field() {
        let json = r#"{"results":[{}]}"#;
        let r: ReleasesResponse = serde_json::from_str(json).unwrap();
        assert!(r.results[0].metadata.is_none());
    }

    #[test]
    fn releases_response_empty_results() {
        let json = r#"{"results":[]}"#;
        let r: ReleasesResponse = serde_json::from_str(json).unwrap();
        assert!(r.results.is_empty());
    }

    // ── SeriesRecord / map_series ─────────────────────────────────────────────

    #[test]
    fn series_record_numeric_series_id() {
        let json = r#"{"series_id":999,"title":"Test Manga","description":null,"image":null,"type":null,"year":null,"status":null,"authors":null,"genres":null}"#;
        let rec: SeriesRecord = serde_json::from_str(json).unwrap();
        assert_eq!(rec.series_id, 999);
    }

    #[test]
    fn series_record_string_series_id() {
        let json = r#"{"series_id":"42","title":"Test","description":null,"image":null,"type":null,"year":null,"status":null,"authors":null,"genres":null}"#;
        let rec: SeriesRecord = serde_json::from_str(json).unwrap();
        assert_eq!(rec.series_id, 42);
    }

    #[test]
    fn map_series_strips_html_description() {
        let json = r#"{"series_id":1,"title":"T","description":"<b>Bold</b> text","image":null,"type":null,"year":null,"status":null,"authors":null,"genres":null}"#;
        let rec: SeriesRecord = serde_json::from_str(json).unwrap();
        let s = map_series(rec);
        assert_eq!(s.description.unwrap(), "Bold text");
    }

    #[test]
    fn map_series_content_type_mapping() {
        for (input, expected) in [
            (Some("Manhwa"), "manhwa"),
            (Some("manhua"), "manhua"),
            (Some("Novel"), "novel"),
            (Some("Light Novel"), "novel"),
            (Some("Web Novel"), "novel"),
            (Some("Manga"), "manga"),
            (None, "manga"),
        ] {
            assert_eq!(map_content_type(input), expected, "failed for {input:?}");
        }
    }

    #[test]
    fn map_series_year_as_string() {
        let json = r#"{"series_id":1,"title":"T","description":null,"image":null,"type":null,"year":"2020","status":null,"authors":null,"genres":null}"#;
        let rec: SeriesRecord = serde_json::from_str(json).unwrap();
        assert_eq!(map_series(rec).year, Some(2020));
    }

    #[test]
    fn map_series_author_prefers_author_type() {
        let json = r#"{"series_id":1,"title":"T","description":null,"image":null,"type":null,"year":null,"status":null,"authors":[{"name":"Artist","type":"Artist"},{"name":"Writer","type":"Author"}],"genres":null}"#;
        let rec: SeriesRecord = serde_json::from_str(json).unwrap();
        assert_eq!(map_series(rec).author.unwrap(), "Writer");
    }

    #[test]
    fn map_series_explicit_genre_normalised() {
        let json = r#"{"series_id":1,"title":"T","description":null,"image":null,"type":null,"year":null,"status":null,"authors":null,"genres":[{"genre":"Action"},{"genre":"Hentai"},{"genre":"Smut"}]}"#;
        let rec: SeriesRecord = serde_json::from_str(json).unwrap();
        let tags = map_series(rec).tags.unwrap();
        // "Hentai" stays as "hentai" (used for source routing); other explicit tags → "adult"
        assert!(tags.contains("hentai"), "expected 'hentai' in tags, got: {tags}");
        assert!(tags.contains("adult"), "expected 'adult' (from Smut) in tags, got: {tags}");
        assert!(tags.contains("Action"));
    }

    // ── strip_html ────────────────────────────────────────────────────────────

    #[test]
    fn strip_html_removes_tags() {
        assert_eq!(strip_html("<b>hello</b> <i>world</i>"), "hello world");
    }

    #[test]
    fn strip_html_plain_text_unchanged() {
        assert_eq!(strip_html("plain text"), "plain text");
    }

    #[test]
    fn strip_html_trims_whitespace() {
        assert_eq!(strip_html("  <p>hi</p>  "), "hi");
    }
}
