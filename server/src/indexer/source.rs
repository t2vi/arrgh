use anyhow::Result;
use async_trait::async_trait;
use sqlx::SqlitePool;

pub struct MangaMeta {
    pub description: Option<String>,
    pub cover_url: Option<String>,
    pub chapter_count: usize,
    pub tags: Option<String>,
}

#[derive(Clone)]
pub struct MangaResult {
    pub id: String,
    pub title: String,
    pub description: Option<String>,
    pub cover_url: Option<String>,
    pub status: String,
    pub author: Option<String>,
    pub year: Option<i64>,
    pub tags: Option<String>,
    /// Content type reported by the plugin per-result, if known.
    pub content_type: Option<String>,
}

/// A page image URL with any HTTP headers the downloader must send to fetch it.
#[derive(Clone)]
pub struct PageUrl {
    pub url: String,
    /// Referer header value, if the CDN requires it.
    pub referer: Option<String>,
}

impl PageUrl {
    pub fn new(url: impl Into<String>) -> Self {
        Self { url: url.into(), referer: None }
    }

    pub fn with_referer(url: impl Into<String>, referer: impl Into<String>) -> Self {
        Self { url: url.into(), referer: Some(referer.into()) }
    }
}

#[async_trait]
pub trait Source: Send + Sync {
    fn id(&self) -> &str;

    /// Display name shown in Settings source list.
    fn name(&self) -> &str { self.id() }

    /// Content types this source carries, e.g. ["manga"], ["manhwa"], ["manga","manhwa","manhua"].
    fn content_types(&self) -> Vec<String> { vec!["manga".to_string()] }

    fn default_explicit(&self) -> bool { false }

    async fn search(&self, query: &str) -> Result<Vec<MangaResult>>;

    async fn sync_chapters(
        &self,
        db: &SqlitePool,
        manga_id: &str,
        source_id: &str,
    ) -> Result<usize>;

    /// Returns ordered page image URLs for a chapter. Downloader fetches and builds the CBZ.
    async fn get_page_urls(&self, chapter_source_id: &str) -> Result<Vec<PageUrl>>;

    /// Returns raw Markdown text for a novel chapter. Used when chapter_format = "text".
    async fn get_chapter_text(&self, chapter_source_id: &str) -> Result<String> {
        let _ = chapter_source_id;
        Err(anyhow::anyhow!("get_chapter_text not supported for source: {}", self.id()))
    }

    async fn fetch_cover(&self, url: &str) -> Result<Vec<u8>>;

    async fn trending(&self) -> Result<Vec<MangaResult>> {
        Err(anyhow::anyhow!("trending not supported for source: {}", self.id()))
    }

    async fn fetch_meta(&self, source_id: &str) -> Result<MangaMeta> {
        let _ = source_id;
        Err(anyhow::anyhow!("fetch_meta not supported for source: {}", self.id()))
    }
}

pub fn sanitize_title(s: &str) -> String {
    s.chars()
        .map(|c| match c {
            '/' | '\\' | ':' | '*' | '?' | '"' | '<' | '>' | '|' => '_',
            c => c,
        })
        .collect::<String>()
        .trim()
        .to_string()
}
