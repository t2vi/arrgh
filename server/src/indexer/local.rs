use anyhow::Result;
use chrono::Utc;
use sqlx::SqlitePool;
use std::path::Path;
use tokio::fs;
use uuid::Uuid;

use super::source::sanitize_title;

pub async fn scan(db: &SqlitePool, manga_dir: &str) -> Result<()> {
    tracing::info!("scanning local manga dir: {}", manga_dir);

    let root = Path::new(manga_dir);
    if !root.exists() {
        fs::create_dir_all(root).await?;
        return Ok(());
    }

    let mut entries = fs::read_dir(root).await?;
    while let Some(entry) = entries.next_entry().await? {
        let path = entry.path();
        let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
        // Skip internal directories (e.g. _downloads for MangaDex chapters)
        if name.starts_with('_') {
            continue;
        }
        if path.is_dir() {
            if let Err(e) = index_manga_dir(db, &path).await {
                tracing::warn!("failed to index {:?}: {}", path, e);
            }
        } else if let Some(ext) = path.extension() {
            if matches!(ext.to_str(), Some("cbz") | Some("zip") | Some("cbr")) {
                if let Err(e) = index_archive(db, &path).await {
                    tracing::warn!("failed to index {:?}: {}", path, e);
                }
            }
        }
    }

    Ok(())
}

async fn index_manga_dir(db: &SqlitePool, path: &Path) -> Result<()> {
    let title = path
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("Unknown")
        .to_string();

    let local_path = path.to_string_lossy().to_string();

    upsert_manga(db, &title, &local_path, "local").await?;
    Ok(())
}

async fn index_archive(db: &SqlitePool, path: &Path) -> Result<()> {
    let title = path
        .file_stem()
        .and_then(|n| n.to_str())
        .unwrap_or("Unknown")
        .to_string();

    let local_path = path.to_string_lossy().to_string();
    upsert_manga(db, &title, &local_path, "local").await?;
    Ok(())
}

/// Scan `{manga_dir}/_downloads/` for CBZ files that exist on disk but are not
/// registered in the DB as downloaded. Updates matching chapters automatically.
pub async fn scan_downloads(db: &SqlitePool, manga_dir: &str) -> Result<()> {
    let downloads_dir = Path::new(manga_dir).join("_downloads");
    if !downloads_dir.exists() {
        return Ok(());
    }

    let mangas = sqlx::query!("SELECT id, title FROM manga WHERE source != 'local'")
        .fetch_all(db)
        .await?;

    for manga in mangas {
        let safe = sanitize_title(&manga.title);
        let dir = downloads_dir.join(&safe);
        if !dir.exists() {
            continue;
        }

        let mut entries = fs::read_dir(&dir).await?;
        while let Some(entry) = entries.next_entry().await? {
            let name = entry.file_name().to_string_lossy().to_string();
            if !name.starts_with("Ch. ") || !name.ends_with(".cbz") {
                continue;
            }

            let num_str = &name["Ch. ".len()..name.len() - ".cbz".len()];
            let chapter_num: f64 = match num_str.parse() {
                Ok(n) => n,
                Err(_) => continue,
            };

            let chapter = sqlx::query!(
                "SELECT id FROM chapters WHERE manga_id = ? AND number = ? AND downloaded = 0",
                manga.id, chapter_num
            )
            .fetch_optional(db)
            .await?;

            if let Some(ch) = chapter {
                let path = entry.path().to_string_lossy().to_string();
                let page_count = count_cbz_pages(&path).unwrap_or(0) as i64;
                sqlx::query!(
                    "UPDATE chapters SET downloaded = 1, local_path = ?, page_count = ? WHERE id = ?",
                    path, page_count, ch.id
                )
                .execute(db)
                .await?;
                tracing::info!("imported local CBZ: {} {}", safe, name);
            }
        }
    }

    Ok(())
}

fn count_cbz_pages(path: &str) -> Result<usize> {
    let p = path.to_string();
    let count = tokio::task::block_in_place(|| {
        let file = std::fs::File::open(&p)?;
        let mut archive = zip::ZipArchive::new(file)?;
        let n = (0..archive.len())
            .filter(|&i| {
                archive.by_index(i).ok()
                    .map(|f| {
                        let lower = f.name().to_lowercase();
                        lower.ends_with(".jpg") || lower.ends_with(".jpeg")
                            || lower.ends_with(".png") || lower.ends_with(".webp")
                    })
                    .unwrap_or(false)
            })
            .count();
        anyhow::Ok(n)
    })?;
    Ok(count)
}

async fn upsert_manga(db: &SqlitePool, title: &str, local_path: &str, source: &str) -> Result<()> {
    let now = Utc::now();
    let existing = sqlx::query!(
        "SELECT id FROM manga WHERE local_path = ?",
        local_path
    )
    .fetch_optional(db)
    .await?;

    if existing.is_none() {
        let id = Uuid::new_v4().to_string();
        sqlx::query!(
            r#"INSERT INTO manga (id, title, status, source, local_path, created_at, updated_at)
               VALUES (?, ?, 'unknown', ?, ?, ?, ?)"#,
            id, title, source, local_path, now, now
        )
        .execute(db)
        .await?;
        tracing::info!("indexed new manga: {}", title);
    }

    Ok(())
}
