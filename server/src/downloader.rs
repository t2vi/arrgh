//! Background download worker — port of `Services/DownloaderService.cs`
//! (ADR 0033, S7 #129). Polls `download_queue` for `pending` items every
//! tick, downloads via plugin-host, writes a `.cbz` (page manga) or `.md`
//! (novel text) file, tries the next `chapter_sources` entry (lowest
//! `external_sources.priority` first, ADR 0013) on failure.
//!
//! `PageCacheService` is *not* ported here despite issue #129 mentioning
//! it — its only call site is `Api/Media.cs`, which is S8 (#130)'s scope,
//! and S8's own issue lists "page cache" explicitly. Porting it now would
//! be dead code until Media lands.

use std::io::Write;
use std::path::Path;
use std::time::Duration;

use serde::Deserialize;
use sqlx::SqlitePool;
use time::macros::format_description;
use time::OffsetDateTime;

const USER_AGENT: &str = "arrgh-server/1.0";
const TICK_INTERVAL: Duration = Duration::from_secs(3);

fn ef_timestamp_now() -> String {
    let fmt =
        format_description!("[year]-[month]-[day] [hour]:[minute]:[second].[subsecond digits:6]");
    OffsetDateTime::now_utc()
        .format(&fmt)
        .expect("format is a static valid pattern")
}

/// Runs forever, ticking every 3s — spawned once from `run()`.
pub async fn run_loop(
    pool: SqlitePool,
    http: reqwest::Client,
    plugin_host_url: String,
    download_dir: String,
) {
    run_loop_with_interval(pool, http, plugin_host_url, download_dir, TICK_INTERVAL).await
}

/// Test seam — a shorter interval keeps integration tests fast.
pub async fn run_loop_with_interval(
    pool: SqlitePool,
    http: reqwest::Client,
    plugin_host_url: String,
    download_dir: String,
    interval: Duration,
) {
    loop {
        if let Err(e) = tick(&pool, &http, &plugin_host_url, &download_dir).await {
            tracing::debug!(error = ?e, "downloader tick error");
        }
        tokio::time::sleep(interval).await;
    }
}

/// Claims and processes one pending item, if any. No distributed lock —
/// SQLite serialises writes and only one instance of this loop runs.
async fn tick(
    pool: &SqlitePool,
    http: &reqwest::Client,
    plugin_host_url: &str,
    download_dir: &str,
) -> anyhow::Result<()> {
    let Some(id): Option<String> = sqlx::query_scalar(
        "SELECT id FROM download_queue WHERE status = 'pending' ORDER BY created_at LIMIT 1",
    )
    .fetch_optional(pool)
    .await?
    else {
        return Ok(());
    };

    sqlx::query("UPDATE download_queue SET status = 'downloading', updated_at = ? WHERE id = ?")
        .bind(ef_timestamp_now())
        .bind(&id)
        .execute(pool)
        .await?;

    process(pool, http, plugin_host_url, download_dir, &id).await
}

#[derive(sqlx::FromRow)]
struct ChapterInfo {
    chapter_format: String,
    content_type: String,
    download_dir: Option<String>,
    title_name: String,
}

async fn process(
    pool: &SqlitePool,
    http: &reqwest::Client,
    plugin_host_url: &str,
    default_download_dir: &str,
    queue_id: &str,
) -> anyhow::Result<()> {
    let (chapter_id, chapter_num): (String, f64) =
        sqlx::query_as("SELECT chapter_id, chapter_num FROM download_queue WHERE id = ?")
            .bind(queue_id)
            .fetch_one(pool)
            .await?;

    let info: Option<ChapterInfo> = sqlx::query_as(
        "SELECT c.chapter_format, t.content_type, t.download_dir, t.title AS title_name \
         FROM chapters c JOIN titles t ON t.id = c.title_id WHERE c.id = ?",
    )
    .bind(&chapter_id)
    .fetch_optional(pool)
    .await?;

    let Some(info) = info else {
        fail(pool, queue_id, "chapter not found").await?;
        return Ok(());
    };

    // Sources ordered by external_sources.priority (lower = preferred, ADR 0013).
    let sources: Vec<(String, String, i64)> = sqlx::query_as(
        "SELECT cs.source, cs.source_id, COALESCE(MIN(es.priority), 100) AS priority \
         FROM chapter_sources cs LEFT JOIN external_sources es ON es.source_key = cs.source \
         WHERE cs.chapter_id = ? GROUP BY cs.source, cs.source_id ORDER BY priority",
    )
    .bind(&chapter_id)
    .fetch_all(pool)
    .await?;

    if sources.is_empty() {
        fail(pool, queue_id, "no chapter sources").await?;
        return Ok(());
    }

    let is_text = info.chapter_format == "text";
    let ext = if is_text { ".md" } else { ".cbz" };
    let file_name = format!("Ch. {}{ext}", format_chapter_num(chapter_num));
    let dest = match info.download_dir.filter(|d| !d.is_empty()) {
        Some(dir) => Path::new(&dir).join(&file_name),
        None => Path::new(default_download_dir)
            .join(format!("_{}", info.content_type))
            .join(sanitize_title(&info.title_name))
            .join(&file_name),
    };

    let mut last_error: Option<String> = None;
    for (source, source_id, _priority) in &sources {
        let result = if is_text {
            download_text(http, plugin_host_url, source, source_id, &dest)
                .await
                .map(|_| 1u32)
        } else {
            download_cbz(
                http,
                plugin_host_url,
                source,
                source_id,
                &dest,
                pool,
                queue_id,
            )
            .await
        };

        match result {
            Ok(page_count) => {
                let now = ef_timestamp_now();
                let dest_str = dest.to_string_lossy().to_string();
                sqlx::query(
                    "UPDATE chapters SET downloaded = 1, local_path = ?, page_count = ? WHERE id = ?",
                )
                .bind(&dest_str)
                .bind(page_count as i64)
                .bind(&chapter_id)
                .execute(pool)
                .await?;

                sqlx::query(
                    "UPDATE download_queue SET status = 'done', updated_at = ? WHERE id = ?",
                )
                .bind(&now)
                .bind(queue_id)
                .execute(pool)
                .await?;
                return Ok(());
            }
            Err(e) => {
                tracing::debug!(source = %source, chapter_num, error = %e, "source failed");
                last_error = Some(e.to_string());
            }
        }
    }

    fail(
        pool,
        queue_id,
        &last_error.unwrap_or_else(|| "all sources failed".into()),
    )
    .await
}

async fn fail(pool: &SqlitePool, queue_id: &str, error: &str) -> anyhow::Result<()> {
    tracing::error!(queue_id, error, "download failed");
    sqlx::query(
        "UPDATE download_queue SET status = 'error', error = ?, updated_at = ? WHERE id = ?",
    )
    .bind(error)
    .bind(ef_timestamp_now())
    .bind(queue_id)
    .execute(pool)
    .await?;
    Ok(())
}

async fn download_text(
    http: &reqwest::Client,
    plugin_host_url: &str,
    source: &str,
    source_id: &str,
    dest: &Path,
) -> anyhow::Result<()> {
    let url = format!(
        "{}/{}/chapter/{}/text",
        plugin_host_url.trim_end_matches('/'),
        source,
        urlencoding::encode(source_id)
    );
    let res = http
        .get(&url)
        .header("User-Agent", USER_AGENT)
        .send()
        .await?;
    if !res.status().is_success() {
        anyhow::bail!("GET {url} -> {}", res.status());
    }
    let text = res.text().await?;

    if let Some(parent) = dest.parent() {
        tokio::fs::create_dir_all(parent).await?;
    }
    tokio::fs::write(dest, text).await?;
    Ok(())
}

#[derive(Deserialize)]
#[serde(untagged)]
enum PageUrl {
    Bare(String),
    WithReferer {
        url: String,
        referer: Option<String>,
    },
}

async fn download_cbz(
    http: &reqwest::Client,
    plugin_host_url: &str,
    source: &str,
    source_id: &str,
    dest: &Path,
    pool: &SqlitePool,
    queue_id: &str,
) -> anyhow::Result<u32> {
    let url = format!(
        "{}/{}/chapter/{}/pages",
        plugin_host_url.trim_end_matches('/'),
        source,
        urlencoding::encode(source_id)
    );
    let res = http
        .get(&url)
        .header("User-Agent", USER_AGENT)
        .send()
        .await?;
    if !res.status().is_success() {
        anyhow::bail!("GET {url} -> {}", res.status());
    }
    let raw: Vec<PageUrl> = res.json().await?;
    let pages: Vec<(String, Option<String>)> = raw
        .into_iter()
        .map(|p| match p {
            PageUrl::Bare(u) => (u, None),
            PageUrl::WithReferer { url, referer } => (url, referer),
        })
        .collect();

    if pages.is_empty() {
        anyhow::bail!("source returned 0 pages for chapter {source_id}");
    }

    sqlx::query("UPDATE download_queue SET pages_total = ?, pages_downloaded = 0 WHERE id = ?")
        .bind(pages.len() as i64)
        .bind(queue_id)
        .execute(pool)
        .await?;

    if let Some(parent) = dest.parent() {
        tokio::fs::create_dir_all(parent).await?;
    }

    let dest = dest.to_path_buf();
    let downloaded_pages = fetch_all_pages(http, &pages).await?;
    write_cbz(&dest, &downloaded_pages)?;

    for i in 0..pages.len() {
        sqlx::query("UPDATE download_queue SET pages_downloaded = ? WHERE id = ?")
            .bind((i + 1) as i64)
            .bind(queue_id)
            .execute(pool)
            .await?;
    }

    Ok(pages.len() as u32)
}

async fn fetch_all_pages(
    http: &reqwest::Client,
    pages: &[(String, Option<String>)],
) -> anyhow::Result<Vec<(Vec<u8>, String)>> {
    let mut out = Vec::with_capacity(pages.len());
    for (page_url, referer) in pages {
        let mut req = http.get(page_url).header("User-Agent", USER_AGENT);
        if let Some(r) = referer {
            req = req.header("Referer", r.as_str());
        }
        let res = req.send().await?;
        if !res.status().is_success() {
            anyhow::bail!("GET {page_url} -> {}", res.status());
        }
        out.push((res.bytes().await?.to_vec(), page_url.clone()));
    }
    Ok(out)
}

/// Extension mirrors .NET: last dot-segment of the URL path (query string
/// stripped), lowercased, `"jpg"` if there's no dot at all.
fn ext_from_url(url: &str) -> String {
    let path = url.split('?').next().unwrap_or(url);
    match path.rsplit_once('.') {
        Some((_, ext)) if !ext.is_empty() => ext.to_lowercase(),
        _ => "jpg".to_string(),
    }
}

fn write_cbz(dest: &Path, pages: &[(Vec<u8>, String)]) -> anyhow::Result<()> {
    let file = std::fs::File::create(dest)?;
    let mut zip = zip::ZipWriter::new(file);
    let options: zip::write::FileOptions<'_, ()> =
        zip::write::FileOptions::default().compression_method(zip::CompressionMethod::Stored);

    for (i, (bytes, url)) in pages.iter().enumerate() {
        let ext = ext_from_url(url);
        zip.start_file(format!("{i:04}.{ext}"), options)?;
        zip.write_all(bytes)?;
    }
    zip.finish()?;
    Ok(())
}

/// Port of `SanitizeTitle`: invalid filename chars -> `_`, trimmed.
fn sanitize_title(title: &str) -> String {
    let invalid = ['/', '\\', ':', '*', '?', '"', '<', '>', '|'];
    title
        .chars()
        .map(|c| if invalid.contains(&c) { '_' } else { c })
        .collect::<String>()
        .trim_matches(|c| c == '_' || c == ' ')
        .to_string()
}

/// Port of .NET's `{0:0000.#}` custom format: zero-padded to 4 integer
/// digits, one optional decimal (rounded to nearest 0.1, omitted if 0).
fn format_chapter_num(n: f64) -> String {
    let rounded = (n * 10.0).round() / 10.0;
    let int_part = rounded.trunc() as i64;
    let frac = ((rounded - int_part as f64).abs() * 10.0).round() as i64;
    if frac == 0 {
        format!("{int_part:04}")
    } else {
        format!("{int_part:04}.{frac}")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn format_chapter_num_whole() {
        assert_eq!(format_chapter_num(5.0), "0005");
        assert_eq!(format_chapter_num(12.0), "0012");
    }

    #[test]
    fn format_chapter_num_fractional() {
        assert_eq!(format_chapter_num(5.5), "0005.5");
        assert_eq!(format_chapter_num(12.25), "0012.3");
    }

    #[test]
    fn sanitize_title_replaces_invalid_chars() {
        assert_eq!(sanitize_title("One Piece: Ch/1"), "One Piece_ Ch_1");
    }

    #[test]
    fn sanitize_title_trims_edges() {
        assert_eq!(sanitize_title("  /weird/  "), "weird");
    }
}
