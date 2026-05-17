use anyhow::{anyhow, Result};
use async_trait::async_trait;
use chrono::Utc;
use scraper::{Html, Selector};
use sqlx::SqlitePool;
use std::sync::OnceLock;
use uuid::Uuid;

use super::source::{MangaMeta, MangaResult, PageUrl, Source};

// ——— Source impl ———

pub struct Mangapill;

#[async_trait]
impl Source for Mangapill {
    fn id(&self) -> &str { "mangapill" }

    async fn search(&self, query: &str) -> Result<Vec<MangaResult>> {
        search(query).await
    }

    async fn sync_chapters(&self, db: &SqlitePool, manga_id: &str, source_id: &str) -> Result<usize> {
        sync_chapters(db, manga_id, source_id).await
    }

    async fn get_page_urls(&self, source_id: &str) -> Result<Vec<PageUrl>> {
        let urls = fetch_chapter_images(source_id).await?;
        Ok(urls.into_iter()
            .map(|u| PageUrl::with_referer(u, BASE))
            .collect())
    }

    async fn fetch_cover(&self, url: &str) -> Result<Vec<u8>> {
        fetch_cover_bytes(url).await
    }

    async fn trending(&self) -> Result<Vec<MangaResult>> {
        trending().await
    }

    async fn fetch_meta(&self, source_id: &str) -> Result<MangaMeta> {
        fetch_meta(source_id).await
    }
}

const BASE: &str = "https://mangapill.com";

static CLIENT: OnceLock<reqwest::Client> = OnceLock::new();

// ——— Cached selectors ———

static SEL_DESC:          OnceLock<Selector> = OnceLock::new();
static SEL_DATA_IMG:      OnceLock<Selector> = OnceLock::new();
static SEL_CHAPTER_LINK:  OnceLock<Selector> = OnceLock::new();
static SEL_TRENDING_CARD: OnceLock<Selector> = OnceLock::new();
static SEL_MANGA_LINK:    OnceLock<Selector> = OnceLock::new();
static SEL_TITLE:         OnceLock<Selector> = OnceLock::new();
static SEL_CHIP:          OnceLock<Selector> = OnceLock::new();
static SEL_SEARCH_CARD:   OnceLock<Selector> = OnceLock::new();
static SEL_STATUS_CHIP:   OnceLock<Selector> = OnceLock::new();
static SEL_YEAR_CHIP:     OnceLock<Selector> = OnceLock::new();
static SEL_TAG_CHIP:      OnceLock<Selector> = OnceLock::new();

fn sel_desc()          -> &'static Selector { SEL_DESC.get_or_init(|| Selector::parse("p.text-sm.text--secondary").unwrap()) }
fn sel_data_img()      -> &'static Selector { SEL_DATA_IMG.get_or_init(|| Selector::parse("img[data-src]").unwrap()) }
fn sel_chapter_link()  -> &'static Selector { SEL_CHAPTER_LINK.get_or_init(|| Selector::parse("a[href^=\"/chapters/\"]").unwrap()) }
fn sel_trending_card() -> &'static Selector { SEL_TRENDING_CARD.get_or_init(|| Selector::parse("div.my-3.grid > div").unwrap()) }
fn sel_manga_link()    -> &'static Selector { SEL_MANGA_LINK.get_or_init(|| Selector::parse("a[href^=\"/manga/\"]").unwrap()) }
fn sel_title()         -> &'static Selector { SEL_TITLE.get_or_init(|| Selector::parse("div.font-black").unwrap()) }
fn sel_chip()          -> &'static Selector { SEL_CHIP.get_or_init(|| Selector::parse("div.bg-card.rounded").unwrap()) }
fn sel_search_card()   -> &'static Selector { SEL_SEARCH_CARD.get_or_init(|| Selector::parse("div.my-3 > div").unwrap()) }
fn sel_status_chip()   -> &'static Selector { SEL_STATUS_CHIP.get_or_init(|| Selector::parse("div.bg-green-500, div.bg-red-500, div.bg-gray-500").unwrap()) }
fn sel_year_chip()     -> &'static Selector { SEL_YEAR_CHIP.get_or_init(|| Selector::parse("div.bg-orange-500").unwrap()) }
fn sel_tag_chip()      -> &'static Selector { SEL_TAG_CHIP.get_or_init(|| Selector::parse("div.bg-card").unwrap()) }

fn client() -> &'static reqwest::Client {
    CLIENT.get_or_init(|| {
        reqwest::Client::builder()
            .user_agent("Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/124.0.0.0 Safari/537.36")
            .build()
            .unwrap()
    })
}

// ——— Public API ———

pub async fn fetch_meta(source_id: &str) -> Result<MangaMeta> {
    let html = fetch_text(&format!("{}/manga/{}", BASE, source_id)).await?;

    let meta = {
        let doc = Html::parse_document(&html);

        let description = doc.select(sel_desc()).next()
            .map(|el| el.text().collect::<String>().trim().to_string())
            .filter(|s| !s.is_empty());

        let cover_url = doc.select(sel_data_img()).next()
            .and_then(|el| el.attr("data-src").map(str::to_string));

        let chapter_count = doc.select(sel_chapter_link()).count();

        MangaMeta { description, cover_url, chapter_count, tags: None }
    };

    Ok(meta)
}

pub async fn trending() -> Result<Vec<MangaResult>> {
    let html = fetch_text(BASE).await?;
    let results = parse_homepage_trending(&html)?;
    if results.is_empty() {
        return Err(anyhow!("mangapill: no trending manga found on homepage"));
    }
    Ok(results)
}

fn parse_homepage_trending(html: &str) -> Result<Vec<MangaResult>> {
    let doc = Html::parse_document(html);

    let mut seen = std::collections::HashSet::new();
    let mut results = Vec::new();

    for card in doc.select(sel_trending_card()) {
        let link_el = match card.select(sel_manga_link()).next() {
            Some(el) => el,
            None => continue,
        };
        let href = match link_el.attr("href") {
            Some(h) => h,
            None => continue,
        };
        // href = "/manga/1/berserk"
        let id_slug = href.trim_start_matches("/manga/").to_string();
        if id_slug.is_empty() || !seen.insert(id_slug.clone()) {
            continue;
        }

        let cover_url = card.select(sel_data_img()).next()
            .and_then(|el| el.attr("data-src"))
            .map(str::to_string);

        let title = card.select(sel_title()).next()
            .map(|el| el.text().collect::<String>().trim().to_string())
            .unwrap_or_default();
        if title.is_empty() {
            continue;
        }

        // Chips hold type, year, and status (e.g. "manga", "1989", "publishing")
        let chips: Vec<String> = card.select(sel_chip())
            .map(|el| el.text().collect::<String>().trim().to_string())
            .filter(|s| !s.is_empty())
            .collect();

        let year: Option<i64> = chips.iter()
            .find_map(|s| s.parse::<i64>().ok().filter(|&y| y > 1900 && y < 2100));

        let status = chips.iter()
            .find(|s| matches!(s.as_str(), "ongoing" | "completed" | "cancelled" | "hiatus" | "publishing"))
            .cloned()
            .unwrap_or_else(|| "unknown".to_string());

        results.push(MangaResult {
            id: id_slug,
            title,
            description: None,
            cover_url,
            status,
            author: None,
            year,
            tags: None,
        });

        if results.len() >= 20 {
            break;
        }
    }

    Ok(results)
}

pub async fn search(query: &str) -> Result<Vec<MangaResult>> {
    let html = fetch_text(&format!("{}/search?q={}&type=&status=", BASE, urlenc(query))).await?;
    parse_search(&html)
}

struct ChapterRow {
    source_id: String,
    number: f64,
    title: Option<String>,
}

struct PageDetail {
    description: Option<String>,
    cover_url: Option<String>,
    rows: Vec<ChapterRow>,
}

/// source_id format: "{id}/{slug}", e.g. "2/one-piece"
pub async fn sync_chapters(db: &SqlitePool, manga_id: &str, source_id: &str) -> Result<usize> {
    let html = fetch_text(&format!("{}/manga/{}", BASE, source_id)).await?;

    // Parse synchronously — scraper types are not Send, so collect all data before any await
    let detail: PageDetail = {
        let doc = Html::parse_document(&html);

        let description = doc.select(sel_desc()).next()
            .map(|el| el.text().collect::<String>().trim().to_string())
            .filter(|s| !s.is_empty());

        let cover_url = doc.select(sel_data_img()).next()
            .and_then(|el| el.attr("data-src").map(str::to_string));

        let rows = doc.select(sel_chapter_link())
            .filter_map(|el| {
                let href = el.attr("href")?;
                let chapter_source_id = href
                    .trim_start_matches('/')
                    .trim_start_matches("chapters/")
                    .to_string();
                let number = parse_chapter_number(&chapter_source_id);
                let title_text = el.text().collect::<String>();
                let title = title_text.trim().to_string();
                Some(ChapterRow {
                    source_id: chapter_source_id,
                    number,
                    title: if title.is_empty() { None } else { Some(title) },
                })
            })
            .collect();

        PageDetail { description, cover_url, rows }
    }; // doc and selectors dropped here

    // Fill in description and cover if not already set
    sqlx::query!(
        "UPDATE manga SET \
         description = COALESCE(description, ?), \
         cover_url = COALESCE(cover_url, ?) \
         WHERE id = ?",
        detail.description, detail.cover_url, manga_id
    )
    .execute(db)
    .await?;

    // Subsequent sync if chapters already exist — new ones get flagged is_new = 1
    let has_existing = sqlx::query_scalar!(
        "SELECT COUNT(*) FROM chapters WHERE manga_id = ?",
        manga_id
    )
    .fetch_one(db)
    .await? > 0;

    let count = detail.rows.len();
    let mut new_count = 0usize;

    for row in detail.rows {
        let now = Utc::now();
        let new_id = Uuid::new_v4().to_string();

        let existing = sqlx::query_scalar!(
            "SELECT id FROM chapters WHERE source_id = ?",
            row.source_id
        )
        .fetch_optional(db)
        .await?;

        if let Some(eid) = existing {
            sqlx::query!(
                "UPDATE chapters SET title = ?, number = ? WHERE id = ?",
                row.title, row.number, eid
            )
            .execute(db)
            .await?;
        } else {
            let is_new = has_existing as i64;
            sqlx::query!(
                r#"INSERT INTO chapters (id, manga_id, title, number, volume, source_id, page_count, downloaded, is_new, created_at)
                   VALUES (?, ?, ?, ?, NULL, ?, 0, 0, ?, ?)"#,
                new_id, manga_id, row.title, row.number, row.source_id, is_new, now
            )
            .execute(db)
            .await?;
            if has_existing { new_count += 1; }
        }
    }

    if new_count > 0 {
        tracing::info!("mangapill: {} new chapters for manga {} ({} total)", new_count, manga_id, count);
    }
    Ok(count)
}

pub async fn fetch_cover_bytes(url: &str) -> Result<Vec<u8>> {
    Ok(client()
        .get(url)
        .header("Referer", BASE)
        .send()
        .await?
        .bytes()
        .await?
        .to_vec())
}

// ——— Helpers ———

async fn fetch_text(url: &str) -> Result<String> {
    let resp = client().get(url).send().await?;
    if !resp.status().is_success() {
        return Err(anyhow!("mangapill: {} returned {}", url, resp.status()));
    }
    Ok(resp.text().await?)
}

async fn fetch_chapter_images(chapter_source_id: &str) -> Result<Vec<String>> {
    let html = fetch_text(&format!("{}/chapters/{}", BASE, chapter_source_id)).await?;

    // Parse synchronously — scraper types are not Send
    let urls: Vec<String> = {
        let doc = Html::parse_document(&html);
        doc.select(sel_data_img())
            .filter_map(|el| el.attr("data-src").map(str::to_string))
            .filter(|u| !u.is_empty())
            .collect()
    };

    if urls.is_empty() {
        return Err(anyhow!("no images found for chapter {}", chapter_source_id));
    }
    Ok(urls)
}

fn parse_search(html: &str) -> Result<Vec<MangaResult>> {
    let doc = Html::parse_document(html);

    let mut seen = std::collections::HashSet::new();
    let mut results = Vec::new();

    for card in doc.select(sel_search_card()) {
        let link_el = match card.select(sel_manga_link()).next() {
            Some(el) => el,
            None => continue,
        };

        let href = match link_el.attr("href") {
            Some(h) => h,
            None => continue,
        };
        // href = "/manga/2/one-piece"
        let id_slug = href.trim_start_matches("/manga/").to_string();
        if id_slug.is_empty() || !seen.insert(id_slug.clone()) {
            continue;
        }

        let cover_url = card.select(sel_data_img()).next()
            .and_then(|el| el.attr("data-src"))
            .map(str::to_string);

        let title = card.select(sel_title()).next()
            .map(|el| el.text().collect::<String>().trim().to_string())
            .unwrap_or_default();

        let status = card.select(sel_status_chip()).next()
            .map(|el| el.text().collect::<String>().trim().to_string())
            .unwrap_or_else(|| "unknown".to_string());

        let year: Option<i64> = card.select(sel_year_chip()).next()
            .and_then(|el| el.text().collect::<String>().trim().parse().ok());

        let tags: Vec<String> = card.select(sel_tag_chip())
            .map(|el| el.text().collect::<String>().trim().to_string())
            .filter(|s| !s.is_empty())
            .take(8)
            .collect();

        results.push(MangaResult {
            id: id_slug,
            title,
            description: None,
            cover_url,
            status,
            author: None,
            year,
            tags: if tags.is_empty() { None } else { Some(tags.join(", ")) },
        });
    }

    Ok(results)
}

/// Extract chapter number from a source_id like "2-11182000/one-piece-chapter-1182"
/// Encoding: chapter_int = 10_000_000 + (chapter_num * 1000)
fn parse_chapter_number(source_id: &str) -> f64 {
    // Try slug first: "one-piece-chapter-1182" → 1182.0
    if let Some(slug_part) = source_id.split('/').nth(1) {
        if let Some(num_str) = slug_part.rsplit("-chapter-").next() {
            if let Ok(n) = num_str.parse::<f64>() {
                return n;
            }
        }
    }

    // Fallback: parse from ID part "2-11182000"
    if let Some(id_part) = source_id.split('/').next() {
        if let Some(num_str) = id_part.split('-').nth(1) {
            if let Ok(n) = num_str.parse::<i64>() {
                let chapter_num = (n - 10_000_000) as f64 / 1000.0;
                if chapter_num >= 0.0 {
                    return chapter_num;
                }
            }
        }
    }

    0.0
}

fn urlenc(s: &str) -> String {
    s.chars()
        .map(|c| match c {
            ' ' => '+',
            c if c.is_alphanumeric() || c == '-' || c == '_' || c == '.' => c,
            _ => '_',
        })
        .collect()
}
