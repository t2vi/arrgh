use anyhow::{anyhow, Result};
use async_trait::async_trait;
use chrono::Utc;
use reqwest::Client;
use serde::Deserialize;
use sqlx::SqlitePool;
use uuid::Uuid;

use super::source::{MangaMeta, MangaResult, PageUrl, Source};

// ── Wire types (plugin protocol) ─────────────────────────────────────────────

#[derive(Deserialize)]
struct InfoResponse {
    id: String,
    name: String,
    #[serde(default)]
    default_explicit: bool,
    #[serde(default)]
    content_types: Vec<String>,
}

#[derive(Deserialize)]
struct SearchItem {
    id: String,
    title: String,
    description: Option<String>,
    cover_url: Option<String>,
    status: String,
    author: Option<String>,
    year: Option<i64>,
    tags: Option<String>,
    content_type: Option<String>,
}

#[derive(Deserialize)]
struct ChapterItem {
    source_id: String,
    number: f64,
    volume: Option<f64>,
    title: Option<String>,
    #[serde(default = "default_chapter_format")]
    chapter_format: String,
}

fn default_chapter_format() -> String { "pages".to_string() }

#[derive(Deserialize)]
struct MetaResponse {
    description: Option<String>,
    cover_url: Option<String>,
    chapter_count: usize,
    #[serde(default)]
    tags: Option<String>,
}

// ── ExternalSource ────────────────────────────────────────────────────────────

pub struct ExternalSource {
    #[allow(dead_code)]
    pub db_id: String,
    pub source_id: String,
    pub display_name: String,
    pub base_url: String,
    pub api_key: Option<String>,
    pub content_types: Vec<String>,
    pub default_explicit: bool,
    client: Client,
}

impl ExternalSource {
    pub fn new(
        db_id: String,
        source_id: String,
        display_name: String,
        base_url: String,
        api_key: Option<String>,
        content_types: Vec<String>,
        default_explicit: bool,
    ) -> Self {
        Self {
            db_id,
            source_id,
            display_name,
            base_url: base_url.trim_end_matches('/').to_string(),
            api_key,
            content_types,
            default_explicit,
            client: Client::new(),
        }
    }

    /// Probe the plugin's /info endpoint and build an ExternalSource from the response.
    pub async fn probe(db_id: String, base_url: String, api_key: Option<String>) -> Result<Self> {
        let client = Client::new();
        let url = format!("{}/info", base_url.trim_end_matches('/'));
        let mut req = client.get(&url);
        if let Some(ref key) = api_key {
            req = req.bearer_auth(key);
        }
        let info: InfoResponse = req.send().await?.error_for_status()?.json().await?;
        Ok(Self::new(
            db_id,
            info.id,
            info.name,
            base_url,
            api_key,
            if info.content_types.is_empty() { vec!["manga".to_string()] } else { info.content_types },
            info.default_explicit,
        ))
    }

    fn get(&self, path: &str) -> reqwest::RequestBuilder {
        let url = format!("{}{}", self.base_url, path);
        let mut req = self.client.get(url);
        if let Some(ref key) = self.api_key {
            req = req.bearer_auth(key);
        }
        req
    }
}

#[async_trait]
impl Source for ExternalSource {
    fn id(&self) -> &str { &self.source_id }
    fn name(&self) -> &str { &self.display_name }
    fn content_types(&self) -> Vec<String> { self.content_types.clone() }
    fn default_explicit(&self) -> bool { self.default_explicit }

    async fn search(&self, query: &str) -> Result<Vec<MangaResult>> {
        let items: Vec<SearchItem> = self.get("/search")
            .query(&[("q", query)])
            .send().await?
            .error_for_status()?.json().await?;

        let default_ct = self.content_types.first().cloned();
        Ok(items.into_iter().map(|i| MangaResult {
            id: i.id,
            title: i.title,
            description: i.description,
            cover_url: i.cover_url,
            status: i.status,
            author: i.author,
            year: i.year,
            tags: i.tags,
            content_type: i.content_type.or_else(|| default_ct.clone()),
        }).collect())
    }

    async fn sync_chapters(&self, db: &SqlitePool, manga_id: &str, source_id: &str) -> Result<usize> {
        let path = format!("/manga/{}/chapters", urlencoding::encode(source_id));
        let chapters: Vec<ChapterItem> = self.get(&path)
            .send().await?
            .error_for_status()?.json().await?;

        let count = chapters.len();
        let now = Utc::now();
        let src_id = self.source_id.clone();
        let mut new_count = 0usize;

        let mut tx = db.begin().await?;
        for ch in &chapters {
            let existing = sqlx::query_scalar!(
                "SELECT id FROM chapters WHERE manga_id = ? AND source_id = ?",
                manga_id, ch.source_id
            )
            .fetch_optional(&mut *tx)
            .await?;

            if existing.is_none() {
                let id = Uuid::new_v4().to_string();
                let num = ch.number;
                let vol = ch.volume;
                let fmt = &ch.chapter_format;
                sqlx::query!(
                    r#"INSERT INTO chapters (id, manga_id, title, number, volume, source_id, page_count, downloaded, chapter_format, created_at)
                       VALUES (?, ?, ?, ?, ?, ?, 0, 0, ?, ?)"#,
                    id, manga_id, ch.title, num, vol, ch.source_id, fmt, now
                )
                .execute(&mut *tx)
                .await?;
                new_count += 1;
            }
        }
        tx.commit().await?;

        if new_count > 0 {
            tracing::info!("{}: {} new chapters for manga {} ({} total)", src_id, new_count, manga_id, count);
        }
        Ok(count)
    }

    async fn get_page_urls(&self, chapter_source_id: &str) -> Result<Vec<PageUrl>> {
        #[derive(Deserialize)]
        #[serde(untagged)]
        enum PageEntry {
            Simple(String),
            Full { url: String, referer: Option<String> },
        }

        let path = format!("/chapter/{}/pages", urlencoding::encode(chapter_source_id));
        let entries: Vec<PageEntry> = self.get(&path)
            .send().await?
            .error_for_status()?.json().await?;

        Ok(entries.into_iter().map(|e| match e {
            PageEntry::Simple(url) => PageUrl::new(url),
            PageEntry::Full { url, referer: Some(r) } => PageUrl::with_referer(url, r),
            PageEntry::Full { url, referer: None } => PageUrl::new(url),
        }).collect())
    }

    async fn fetch_cover(&self, url: &str) -> Result<Vec<u8>> {
        Ok(self.client.get(url).send().await?.bytes().await?.to_vec())
    }

    async fn trending(&self) -> Result<Vec<MangaResult>> {
        let resp = self.get("/trending").send().await?;
        if resp.status() == reqwest::StatusCode::NOT_FOUND
            || resp.status() == reqwest::StatusCode::METHOD_NOT_ALLOWED {
            return Err(anyhow!("trending not supported by source: {}", self.source_id));
        }
        let items: Vec<SearchItem> = resp.error_for_status()?.json().await?;
        let default_ct = self.content_types.first().cloned();
        Ok(items.into_iter().map(|i| MangaResult {
            id: i.id, title: i.title, description: i.description,
            cover_url: i.cover_url, status: i.status,
            author: i.author, year: i.year, tags: i.tags,
            content_type: i.content_type.or_else(|| default_ct.clone()),
        }).collect())
    }

    async fn get_chapter_text(&self, chapter_source_id: &str) -> Result<String> {
        let path = format!("/chapter/{}/text", urlencoding::encode(chapter_source_id));
        let resp = self.get(&path).send().await?;
        if resp.status() == reqwest::StatusCode::NOT_FOUND
            || resp.status() == reqwest::StatusCode::METHOD_NOT_ALLOWED {
            return Err(anyhow!("get_chapter_text not supported by source: {}", self.source_id));
        }
        Ok(resp.error_for_status()?.text().await?)
    }

    async fn fetch_meta(&self, source_id: &str) -> Result<MangaMeta> {
        let path = format!("/manga/{}/meta", urlencoding::encode(source_id));
        let resp = self.get(&path).send().await?;
        if resp.status() == reqwest::StatusCode::NOT_FOUND
            || resp.status() == reqwest::StatusCode::METHOD_NOT_ALLOWED {
            return Err(anyhow!("fetch_meta not supported by source: {}", self.source_id));
        }
        let meta: MetaResponse = resp.error_for_status()?.json().await?;
        Ok(MangaMeta {
            description: meta.description,
            cover_url: meta.cover_url,
            chapter_count: meta.chapter_count,
            tags: meta.tags,
        })
    }
}
