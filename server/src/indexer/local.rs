use anyhow::Result;
use chrono::Utc;
use sqlx::SqlitePool;
use std::path::Path;
use tokio::fs;
use uuid::Uuid;

pub async fn verify_title_downloads(db: &SqlitePool, title_id: &str) -> Result<()> {
    let rows = sqlx::query!(
        r#"SELECT id as "id!", local_path FROM chapters
           WHERE title_id = ? AND downloaded = 1 AND local_path IS NOT NULL"#,
        title_id
    )
    .fetch_all(db)
    .await?;

    for row in rows {
        if let Some(path) = row.local_path {
            if !path_has_content(&path).await {
                sqlx::query!(
                    "UPDATE chapters SET downloaded = 0, local_path = NULL WHERE id = ?",
                    row.id
                )
                .execute(db)
                .await?;
                tracing::warn!("reset stale download for chapter {}", row.id);
            }
        }
    }
    Ok(())
}

pub async fn verify_downloads(db: &SqlitePool) -> Result<()> {
    let rows = sqlx::query!(
        r#"SELECT id as "id!", local_path FROM chapters WHERE downloaded = 1 AND local_path IS NOT NULL"#
    )
    .fetch_all(db)
    .await?;

    let mut reset = 0u32;
    for row in rows {
        if let Some(path) = row.local_path {
            if !path_has_content(&path).await {
                sqlx::query!(
                    "UPDATE chapters SET downloaded = 0, local_path = NULL WHERE id = ?",
                    row.id
                )
                .execute(db)
                .await?;
                reset += 1;
            }
        }
    }

    if reset > 0 {
        tracing::warn!("startup: reset {} chapters whose files were missing", reset);
    }
    Ok(())
}

async fn path_has_content(path: &str) -> bool {
    let p = Path::new(path);
    if !p.exists() {
        return false;
    }
    let ext = p.extension().and_then(|e| e.to_str()).unwrap_or("").to_lowercase();
    if ext == "cbz" || ext == "zip" || ext == "md" {
        return true;
    }
    // directory: must contain at least one image file
    if let Ok(mut dir) = fs::read_dir(path).await {
        while let Ok(Some(entry)) = dir.next_entry().await {
            let name = entry.file_name().to_string_lossy().to_lowercase();
            if name.ends_with(".jpg") || name.ends_with(".jpeg")
                || name.ends_with(".png") || name.ends_with(".webp")
                || name.ends_with(".avif")
            {
                return true;
            }
        }
    }
    false
}

use super::source::sanitize_title;

pub async fn scan(db: &SqlitePool, download_dir: &str) -> Result<()> {
    tracing::info!("scanning local title dir: {}", download_dir);

    let root = Path::new(download_dir);
    if !root.exists() {
        fs::create_dir_all(root).await?;
        return Ok(());
    }

    let mut entries = fs::read_dir(root).await?;
    while let Some(entry) = entries.next_entry().await? {
        let path = entry.path();
        let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
        // Skip internal directories
        if name.starts_with('_') {
            continue;
        }
        if path.is_dir() {
            if let Err(e) = index_download_dir(db, &path).await {
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

async fn index_download_dir(db: &SqlitePool, path: &Path) -> Result<()> {
    let title = path
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("Unknown")
        .to_string();

    let local_path = path.to_string_lossy().to_string();

    upsert_title(db, &title, &local_path).await?;
    Ok(())
}

async fn index_archive(db: &SqlitePool, path: &Path) -> Result<()> {
    let title = path
        .file_stem()
        .and_then(|n| n.to_str())
        .unwrap_or("Unknown")
        .to_string();

    let local_path = path.to_string_lossy().to_string();
    upsert_title(db, &title, &local_path).await?;
    Ok(())
}

/// Scan `{download_dir}/_{content_type}/` for CBZ/MD files not registered in the DB as downloaded.
pub async fn scan_downloads(db: &SqlitePool, download_dir: &str) -> Result<()> {
    let titles = sqlx::query!(
        "SELECT id, title, content_type FROM titles WHERE EXISTS (SELECT 1 FROM title_sources WHERE title_id = titles.id)"
    )
    .fetch_all(db)
    .await?;

    for title in titles {
        let safe = sanitize_title(&title.title);
        let content_type = title.content_type.as_str();

        let candidate_dirs = [
            Path::new(download_dir).join(format!("_{content_type}")).join(&safe),
        ];

        for dir in &candidate_dirs {
            if !dir.exists() {
                continue;
            }

            let mut entries = match fs::read_dir(dir).await {
                Ok(e) => e,
                Err(_) => continue,
            };
            while let Some(entry) = entries.next_entry().await? {
                let name = entry.file_name().to_string_lossy().to_string();
                if !name.starts_with("Ch. ") {
                    continue;
                }

                let (num_str, is_novel) = if name.ends_with(".cbz") {
                    (&name["Ch. ".len()..name.len() - ".cbz".len()], false)
                } else if name.ends_with(".md") {
                    (&name["Ch. ".len()..name.len() - ".md".len()], true)
                } else {
                    continue;
                };

                let chapter_num: f64 = match num_str.parse() {
                    Ok(n) => n,
                    Err(_) => continue,
                };

                let chapter = sqlx::query!(
                    "SELECT id FROM chapters WHERE title_id = ? AND number = ? AND downloaded = 0",
                    title.id, chapter_num
                )
                .fetch_optional(db)
                .await?;

                if let Some(ch) = chapter {
                    let path = entry.path().to_string_lossy().to_string();
                    if is_novel {
                        sqlx::query!(
                            "UPDATE chapters SET downloaded = 1, local_path = ? WHERE id = ?",
                            path, ch.id
                        )
                        .execute(db)
                        .await?;
                        tracing::info!("imported local novel MD: {} {}", safe, name);
                    } else {
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

async fn upsert_title(db: &SqlitePool, title: &str, local_path: &str) -> Result<()> {
    let now = Utc::now();
    let existing = sqlx::query!(
        "SELECT id FROM titles WHERE local_path = ?",
        local_path
    )
    .fetch_optional(db)
    .await?;

    if existing.is_none() {
        let id = Uuid::new_v4().to_string();
        sqlx::query!(
            r#"INSERT INTO titles (id, title, status, local_path, created_at, updated_at)
               VALUES (?, ?, 'unknown', ?, ?, ?)"#,
            id, title, local_path, now, now
        )
        .execute(db)
        .await?;
        tracing::info!("indexed new title: {}", title);
    }

    Ok(())
}
