use anyhow::Result;
use reqwest::Client;
use serde::Deserialize;

const BASE: &str = "https://api.mangaupdates.com/v1";

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
                    if ["adult", "hentai", "smut", "18+", "erotic"]
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
        let resp = self
            .client
            .post(format!("{BASE}/releases/search"))
            .json(&body)
            .send()
            .await?
            .error_for_status()?
            .json::<ReleasesResponse>()
            .await?;

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
