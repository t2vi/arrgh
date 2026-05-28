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

    async fn sync_chapters(&self, db: &SqlitePool, title_id: &str, source_id: &str) -> Result<usize> {
        let path = format!("/manga/{}/chapters", urlencoding::encode(source_id));
        let resp = self.get(&path).send().await?;
        if resp.status() == reqwest::StatusCode::BAD_GATEWAY {
            return Err(anyhow!("{}: source temporarily unavailable (502)", self.source_id));
        }
        let chapters: Vec<ChapterItem> = resp.error_for_status()?.json().await?;

        let count = chapters.len();
        let now = Utc::now();
        let src_key = self.source_id.clone();
        let mut new_count = 0usize;

        let mut tx = db.begin().await?;
        for ch in &chapters {
            // Dedup by (title_id, number) — same float number = same logical chapter
            let chapter_id: Option<String> = sqlx::query_scalar!(
                r#"SELECT id as "id!" FROM chapters WHERE title_id = ? AND number = ?"#,
                title_id, ch.number
            )
            .fetch_optional(&mut *tx)
            .await?;

            let chapter_id = match chapter_id {
                Some(id) => id,
                None => {
                    let id = Uuid::new_v4().to_string();
                    let num = ch.number;
                    let vol = ch.volume;
                    let fmt = &ch.chapter_format;
                    sqlx::query!(
                        r#"INSERT INTO chapters (id, title_id, title, number, volume, page_count, downloaded, chapter_format, created_at)
                           VALUES (?, ?, ?, ?, ?, 0, 0, ?, ?)"#,
                        id, title_id, ch.title, num, vol, fmt, now
                    )
                    .execute(&mut *tx)
                    .await?;
                    new_count += 1;
                    id
                }
            };

            // Upsert source link for this chapter
            let cs_id = Uuid::new_v4().to_string();
            sqlx::query!(
                r#"INSERT INTO chapter_sources (id, chapter_id, source, source_id)
                   VALUES (?, ?, ?, ?)
                   ON CONFLICT(chapter_id, source) DO UPDATE SET source_id = excluded.source_id"#,
                cs_id, chapter_id, src_key, ch.source_id
            )
            .execute(&mut *tx)
            .await?;
        }
        tx.commit().await?;

        if new_count > 0 {
            tracing::info!("{}: {} new chapters for title {} ({} total)", src_key, new_count, title_id, count);
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
        let resp = self.get(&format!("/cover?url={}", urlencoding::encode(url))).send().await?;
        match resp.status() {
            s if s.is_success() => return Ok(resp.bytes().await?.to_vec()),
            reqwest::StatusCode::NOT_IMPLEMENTED => {} // plugin has no /cover → fall through
            s => return Err(anyhow!("cover proxy returned {}", s)),
        }
        Ok(self.client.get(url).header("User-Agent", "Mozilla/5.0").send().await?.bytes().await?.to_vec())
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

#[cfg(test)]
mod tests {
    use super::*;
    use axum::{Json, Router, routing::get};
    use axum::http::StatusCode as AxumStatus;
    use serde_json::{Value, json};
    use sqlx::sqlite::{SqliteConnectOptions, SqlitePoolOptions};
    use std::sync::Arc;
    use tokio::net::TcpListener;

    async fn make_pool() -> SqlitePool {
        let opts = SqliteConnectOptions::new()
            .filename(":memory:")
            .create_if_missing(true);
        let pool = SqlitePoolOptions::new()
            .max_connections(1)
            .connect_with(opts)
            .await
            .unwrap();
        sqlx::migrate!("./migrations").run(&pool).await.unwrap();
        pool
    }

    async fn spawn_502_chapters_server() -> String {
        let router = Router::new().route(
            "/manga/{source_id}/chapters",
            get(|| async { AxumStatus::BAD_GATEWAY }),
        );
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let port = listener.local_addr().unwrap().port();
        tokio::spawn(async move { axum::serve(listener, router).await.unwrap() });
        format!("http://127.0.0.1:{port}")
    }

    async fn spawn_chapters_server(chapters: Value) -> String {
        let chapters = Arc::new(chapters);
        let router = Router::new().route(
            "/manga/{source_id}/chapters",
            get(move || {
                let data = chapters.clone();
                async move { Json(data.as_ref().clone()) }
            }),
        );
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let port = listener.local_addr().unwrap().port();
        tokio::spawn(async move { axum::serve(listener, router).await.unwrap() });
        format!("http://127.0.0.1:{port}")
    }

    async fn seed_manga(pool: &SqlitePool, id: &str) {
        let now = chrono::Utc::now().to_rfc3339();
        sqlx::query(
            "INSERT INTO titles (id, title, status, sync_status, content_type, is_explicit, created_at, updated_at) \
             VALUES (?, 'Test', 'unknown', 'ready', 'manga', 0, ?, ?)"
        )
        .bind(id).bind(&now).bind(&now)
        .execute(pool).await.unwrap();
    }

    // Two sources with overlapping chapter numbers → 4 chapters, 6 chapter_sources rows
    #[tokio::test]
    async fn sync_chapters_deduplicates_by_number() {
        let pool = make_pool().await;
        seed_manga(&pool, "m-dedup").await;

        let base_a = spawn_chapters_server(json!([
            {"source_id": "a-ch1", "number": 1.0, "chapter_format": "pages"},
            {"source_id": "a-ch2", "number": 2.0, "chapter_format": "pages"},
            {"source_id": "a-ch3", "number": 3.0, "chapter_format": "pages"},
        ])).await;
        let src_a = ExternalSource::new(
            "db-a".into(), "source-a".into(), "Source A".into(),
            base_a, None, vec!["manga".into()], false,
        );
        src_a.sync_chapters(&pool, "m-dedup", "manga-src-id").await.unwrap();

        let base_b = spawn_chapters_server(json!([
            {"source_id": "b-ch2", "number": 2.0, "chapter_format": "pages"},
            {"source_id": "b-ch3", "number": 3.0, "chapter_format": "pages"},
            {"source_id": "b-ch4", "number": 4.0, "chapter_format": "pages"},
        ])).await;
        let src_b = ExternalSource::new(
            "db-b".into(), "source-b".into(), "Source B".into(),
            base_b, None, vec!["manga".into()], false,
        );
        src_b.sync_chapters(&pool, "m-dedup", "manga-src-id").await.unwrap();

        let chapter_count: i64 =
            sqlx::query_scalar("SELECT COUNT(*) FROM chapters WHERE title_id = 'm-dedup'")
                .fetch_one(&pool).await.unwrap();
        assert_eq!(chapter_count, 4, "chapters 1-3 from A + ch 4 from B = 4 unique rows");

        let source_count: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM chapter_sources cs \
             JOIN chapters c ON c.id = cs.chapter_id \
             WHERE c.title_id = 'm-dedup'"
        ).fetch_one(&pool).await.unwrap();
        assert_eq!(source_count, 6, "3 from source-a + 3 from source-b = 6 chapter_sources rows");
    }

    // Syncing the same source twice produces no duplicate rows
    #[tokio::test]
    async fn sync_chapters_is_idempotent() {
        let pool = make_pool().await;
        seed_manga(&pool, "m-idem").await;

        let base = spawn_chapters_server(json!([
            {"source_id": "ch1", "number": 1.0, "chapter_format": "pages"},
            {"source_id": "ch2", "number": 2.0, "chapter_format": "pages"},
        ])).await;
        let src = ExternalSource::new(
            "db-idem".into(), "source-idem".into(), "Idempotent".into(),
            base, None, vec!["manga".into()], false,
        );

        src.sync_chapters(&pool, "m-idem", "mid").await.unwrap();
        src.sync_chapters(&pool, "m-idem", "mid").await.unwrap();

        let chapter_count: i64 =
            sqlx::query_scalar("SELECT COUNT(*) FROM chapters WHERE title_id = 'm-idem'")
                .fetch_one(&pool).await.unwrap();
        assert_eq!(chapter_count, 2);

        let source_count: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM chapter_sources cs \
             JOIN chapters c ON c.id = cs.chapter_id \
             WHERE c.title_id = 'm-idem'"
        ).fetch_one(&pool).await.unwrap();
        assert_eq!(source_count, 2);
    }

    // ON CONFLICT updates source_id when plugin returns a new identifier for the same chapter number
    #[tokio::test]
    async fn sync_chapters_updates_source_id_on_conflict() {
        let pool = make_pool().await;
        seed_manga(&pool, "m-update").await;

        let base_v1 = spawn_chapters_server(json!([
            {"source_id": "old-id", "number": 1.0, "chapter_format": "pages"},
        ])).await;
        let src_v1 = ExternalSource::new(
            "db-up".into(), "source-up".into(), "Source".into(),
            base_v1, None, vec!["manga".into()], false,
        );
        src_v1.sync_chapters(&pool, "m-update", "mid").await.unwrap();

        let base_v2 = spawn_chapters_server(json!([
            {"source_id": "new-id", "number": 1.0, "chapter_format": "pages"},
        ])).await;
        let src_v2 = ExternalSource::new(
            "db-up".into(), "source-up".into(), "Source".into(),
            base_v2, None, vec!["manga".into()], false,
        );
        src_v2.sync_chapters(&pool, "m-update", "mid").await.unwrap();

        let stored_id: String = sqlx::query_scalar(
            "SELECT cs.source_id FROM chapter_sources cs \
             JOIN chapters c ON c.id = cs.chapter_id \
             WHERE c.title_id = 'm-update' AND cs.source = 'source-up'"
        ).fetch_one(&pool).await.unwrap();
        assert_eq!(stored_id, "new-id", "ON CONFLICT updates source_id to the new value");

        let row_count: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM chapter_sources cs \
             JOIN chapters c ON c.id = cs.chapter_id \
             WHERE c.title_id = 'm-update'"
        ).fetch_one(&pool).await.unwrap();
        assert_eq!(row_count, 1, "no duplicate rows after update");
    }

    // 502 from plugin-host → Err (source temporarily unavailable)
    #[tokio::test]
    async fn sync_chapters_returns_err_on_502() {
        let pool = make_pool().await;
        seed_manga(&pool, "m-502").await;

        let base = spawn_502_chapters_server().await;
        let src = ExternalSource::new(
            "db-502".into(), "source-502".into(), "Flaky".into(),
            base, None, vec!["manga".into()], false,
        );

        let result = src.sync_chapters(&pool, "m-502", "mid").await;
        assert!(result.is_err(), "502 should propagate as Err");
        assert!(result.unwrap_err().to_string().contains("502"));
    }

    // Existing chapters are preserved when source returns 502 on a subsequent sync
    #[tokio::test]
    async fn sync_chapters_preserves_existing_on_502() {
        let pool = make_pool().await;
        seed_manga(&pool, "m-502-preserve").await;

        let base_ok = spawn_chapters_server(json!([
            {"source_id": "ch1", "number": 1.0, "chapter_format": "pages"},
        ])).await;
        let src_ok = ExternalSource::new(
            "db-pp".into(), "src-pp".into(), "Src".into(),
            base_ok, None, vec!["manga".into()], false,
        );
        src_ok.sync_chapters(&pool, "m-502-preserve", "mid").await.unwrap();

        let base_502 = spawn_502_chapters_server().await;
        let src_502 = ExternalSource::new(
            "db-pp".into(), "src-pp".into(), "Src".into(),
            base_502, None, vec!["manga".into()], false,
        );
        let result = src_502.sync_chapters(&pool, "m-502-preserve", "mid").await;
        assert!(result.is_err(), "502 should propagate as Err");

        // Existing chapters from previous successful sync must survive
        let count: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM chapters WHERE title_id = 'm-502-preserve'"
        ).fetch_one(&pool).await.unwrap();
        assert_eq!(count, 1, "chapter from successful sync must survive 502");
    }
}
