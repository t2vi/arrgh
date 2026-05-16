use anyhow::Result;
use async_trait::async_trait;
use sqlx::SqlitePool;
use std::path::Path;

pub struct MangaMeta {
    pub description: Option<String>,
    pub cover_url: Option<String>,
    pub chapter_count: usize,
}

pub struct MangaResult {
    pub id: String,
    pub title: String,
    pub description: Option<String>,
    pub cover_url: Option<String>,
    pub status: String,
    pub author: Option<String>,
    pub year: Option<i64>,
    pub tags: Option<String>,
}

#[async_trait]
pub trait Source: Send + Sync {
    fn id(&self) -> &str;

    async fn search(&self, query: &str) -> Result<Vec<MangaResult>>;

    async fn sync_chapters(
        &self,
        db: &SqlitePool,
        manga_id: &str,
        source_id: &str,
    ) -> Result<usize>;

    async fn download_chapter(&self, source_id: &str, dest: &Path) -> Result<usize>;

    async fn get_page_url(&self, source_id: &str, page: usize) -> Result<String>;

    async fn fetch_cover(&self, url: &str) -> Result<Vec<u8>>;

    async fn trending(&self) -> Result<Vec<MangaResult>> {
        Err(anyhow::anyhow!("trending not supported for source: {}", self.id()))
    }

    async fn fetch_meta(&self, source_id: &str) -> Result<MangaMeta> {
        let _ = source_id;
        Err(anyhow::anyhow!("fetch_meta not supported for source: {}", self.id()))
    }

    fn default_explicit(&self) -> bool {
        false
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
