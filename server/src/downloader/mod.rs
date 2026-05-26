use anyhow::Result;
use chrono::Utc;
use std::io::Write as _;
use std::path::Path;
use std::sync::Arc;
use tokio::sync::Mutex;

use crate::{api::settings::read_settings, indexer::source::sanitize_title, AppState};

pub async fn start_worker(state: AppState) {
    let workers = read_settings(&state.db, &state.config.download_dir).await.download_workers.max(1) as usize;
    let claim_lock: Arc<Mutex<()>> = Arc::new(Mutex::new(()));

    tracing::info!("starting {} download worker(s)", workers);
    for i in 0..workers {
        let state = state.clone();
        let lock = claim_lock.clone();
        tokio::spawn(async move {
            loop {
                if let Err(e) = tick(&state, &lock).await {
                    tracing::error!("download worker {}: {}", i + 1, e);
                }
                tokio::time::sleep(tokio::time::Duration::from_secs(3)).await;
            }
        });
    }
}

async fn tick(state: &AppState, claim_lock: &Mutex<()>) -> Result<()> {
    let now = Utc::now();

    let item = {
        let _guard = claim_lock.lock().await;

        let item = sqlx::query!(
            r#"SELECT id as "id!", chapter_id as "chapter_id!", manga_title as "manga_title!",
                      chapter_num as "chapter_num!"
               FROM download_queue WHERE status = 'pending' ORDER BY created_at LIMIT 1"#
        )
        .fetch_optional(&state.db)
        .await?;

        let Some(item) = item else { return Ok(()) };

        sqlx::query!(
            "UPDATE download_queue SET status = 'downloading', updated_at = ? WHERE id = ?",
            now, item.id
        )
        .execute(&state.db)
        .await?;

        item
    };

    let chapter = sqlx::query!(
        r#"SELECT c.chapter_format as "chapter_format!", m.download_dir, m.content_type as "content_type!"
           FROM chapters c JOIN titles m ON c.title_id = m.id
           WHERE c.id = ?"#,
        item.chapter_id
    )
    .fetch_one(&state.db)
    .await?;

    // Fetch all source links for this chapter, ordered by source priority (lower = higher priority)
    let sources = sqlx::query!(
        r#"SELECT cs.source as "source!", cs.source_id as "source_id!"
           FROM chapter_sources cs
           LEFT JOIN external_sources es ON es.source_key = cs.source
           WHERE cs.chapter_id = ?
           ORDER BY COALESCE(es.priority, 100) ASC"#,
        item.chapter_id
    )
    .fetch_all(&state.db)
    .await?;

    if sources.is_empty() {
        fail(state, &item.id, "chapter has no source links").await?;
        return Ok(());
    }

    let is_text = chapter.chapter_format == "text";
    let file_name = if is_text {
        format!("Ch. {:04.1}.md", item.chapter_num)
    } else {
        format!("Ch. {:04.1}.cbz", item.chapter_num)
    };
    let dest = match chapter.download_dir {
        Some(ref dir) => Path::new(dir).join(&file_name),
        None => {
            let safe = sanitize_title(&item.manga_title);
            Path::new(&state.config.download_dir)
                .join(format!("_{}", &chapter.content_type))
                .join(&safe)
                .join(&file_name)
        }
    };

    // Try each source in priority order until one succeeds
    let mut download_result: Result<usize> = Err(anyhow::anyhow!("no sources attempted"));
    for row in &sources {
        let src = match state.sources.read().await.get(&row.source).cloned() {
            Some(s) => s,
            None => {
                tracing::warn!("no source impl for '{}', skipping", row.source);
                continue;
            }
        };

        tracing::info!("downloading: {} Ch. {} ({})", item.manga_title, item.chapter_num, row.source);

        download_result = if is_text {
            download_text(src.as_ref(), &row.source_id, &dest).await.map(|_| 1usize)
        } else {
            download_cbz(src.as_ref(), &row.source_id, &dest, &state.db, &item.id).await
        };

        if download_result.is_ok() {
            break;
        }
        tracing::warn!("source {} failed for Ch. {}: {:?}", row.source, item.chapter_num, download_result);
    }

    match download_result {
        Ok(page_count) => {
            let path_str = dest.to_string_lossy().to_string();
            let pc = page_count as i64;
            let now2 = Utc::now();
            let mut tx = state.db.begin().await?;
            sqlx::query!(
                "UPDATE chapters SET downloaded = 1, local_path = ?, page_count = ? WHERE id = ?",
                path_str, pc, item.chapter_id
            )
            .execute(&mut *tx)
            .await?;

            sqlx::query!(
                "UPDATE download_queue SET status = 'done', updated_at = ? WHERE id = ?",
                now2, item.id
            )
            .execute(&mut *tx)
            .await?;
            tx.commit().await?;

            if is_text {
                tracing::info!("downloaded chapter text → {:?}", dest);
            } else {
                tracing::info!("downloaded {} pages → {:?}", page_count, dest);
            }

        }
        Err(e) => {
            let err = e.to_string();
            fail(state, &item.id, &err).await?;
            tracing::error!("download failed: {}", err);
        }
    }

    Ok(())
}

async fn download_text(
    src: &dyn crate::indexer::Source,
    chapter_source_id: &str,
    dest: &Path,
) -> Result<()> {
    let text = src.get_chapter_text(chapter_source_id).await?;
    if let Some(parent) = dest.parent() {
        tokio::fs::create_dir_all(parent).await?;
    }
    tokio::fs::write(dest, text.as_bytes()).await?;
    Ok(())
}

async fn download_cbz(
    src: &dyn crate::indexer::Source,
    chapter_source_id: &str,
    dest: &Path,
    db: &sqlx::SqlitePool,
    queue_id: &str,
) -> Result<usize> {
    let pages = src.get_page_urls(chapter_source_id).await?;
    let page_count = pages.len();
    let total = page_count as i64;

    if let Some(parent) = dest.parent() {
        tokio::fs::create_dir_all(parent).await?;
    }

    sqlx::query!(
        "UPDATE download_queue SET pages_total = ?, pages_downloaded = 0 WHERE id = ?",
        total, queue_id
    )
    .execute(db)
    .await?;

    let client = reqwest::Client::new();
    let buf = Vec::<u8>::new();
    let cursor = std::io::Cursor::new(buf);
    let mut zip = zip::ZipWriter::new(cursor);
    let opts = zip::write::SimpleFileOptions::default()
        .compression_method(zip::CompressionMethod::Stored);

    for (i, page) in pages.iter().enumerate() {
        let ext = page.url.split('?').next().unwrap_or(&page.url)
            .rsplit('.').next().unwrap_or("jpg");
        let entry_name = format!("{:04}.{}", i, ext);

        let mut req = client.get(&page.url);
        if let Some(ref referer) = page.referer {
            req = req.header("Referer", referer);
        }
        let bytes = req.send().await?.error_for_status()?.bytes().await?;

        zip.start_file(&entry_name, opts)?;
        zip.write_all(&bytes)?;

        let done = (i + 1) as i64;
        let _ = sqlx::query!(
            "UPDATE download_queue SET pages_downloaded = ? WHERE id = ?",
            done, queue_id
        )
        .execute(db)
        .await;

        tokio::time::sleep(tokio::time::Duration::from_millis(150)).await;
    }

    let cursor = zip.finish()?;
    tokio::fs::write(dest, cursor.into_inner()).await?;
    Ok(page_count)
}

async fn fail(state: &AppState, queue_id: &str, error: &str) -> Result<()> {
    let now = Utc::now();
    sqlx::query!(
        "UPDATE download_queue SET status = 'error', error = ?, updated_at = ? WHERE id = ?",
        error, now, queue_id
    )
    .execute(&state.db)
    .await?;
    Ok(())
}
