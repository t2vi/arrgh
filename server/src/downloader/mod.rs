use anyhow::Result;
use chrono::Utc;
use std::path::Path;
use std::sync::Arc;
use tokio::sync::Mutex;

use crate::{api::settings::read_settings, indexer::source::sanitize_title, AppState};

pub async fn start_worker(state: AppState) {
    let workers = read_settings(&state.db, &state.config.manga_dir).await.download_workers.max(1) as usize;
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

    // Hold mutex only for SELECT+UPDATE claim to prevent two workers picking the same row.
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
    }; // claim_lock released — other workers can now claim their own items

    let chapter = sqlx::query!(
        r#"SELECT c.source_id, m.source as "source!", m.download_dir
           FROM chapters c JOIN manga m ON c.manga_id = m.id
           WHERE c.id = ?"#,
        item.chapter_id
    )
    .fetch_one(&state.db)
    .await?;

    let source_id = match chapter.source_id {
        Some(s) => s,
        None => {
            fail(state, &item.id, "chapter has no source_id").await?;
            return Ok(());
        }
    };

    let cbz_name = format!("Ch. {:04.1}.cbz", item.chapter_num);
    let dest = match chapter.download_dir {
        Some(ref dir) => Path::new(dir).join(&cbz_name),
        None => {
            let safe = sanitize_title(&item.manga_title);
            Path::new(&state.config.manga_dir)
                .join("_downloads")
                .join(&safe)
                .join(&cbz_name)
        }
    };

    tracing::info!("downloading: {} Ch. {} ({})", item.manga_title, item.chapter_num, chapter.source);

    let src = match state.sources.get(&chapter.source) {
        Some(s) => s.clone(),
        None => {
            fail(state, &item.id, &format!("unknown source: {}", chapter.source)).await?;
            return Ok(());
        }
    };

    match src.download_chapter(&source_id, &dest).await {
        Ok(page_count) => {
            let path_str = dest.to_string_lossy().to_string();
            let pc = page_count as i64;
            let now2 = Utc::now();
            sqlx::query!(
                "UPDATE chapters SET downloaded = 1, local_path = ?, page_count = ? WHERE id = ?",
                path_str, pc, item.chapter_id
            )
            .execute(&state.db)
            .await?;

            sqlx::query!(
                "UPDATE download_queue SET status = 'done', updated_at = ? WHERE id = ?",
                now2, item.id
            )
            .execute(&state.db)
            .await?;

            tracing::info!("downloaded {} pages → {:?}", page_count, dest);
        }
        Err(e) => {
            let err = e.to_string();
            fail(state, &item.id, &err).await?;
            tracing::error!("download failed: {}", err);
        }
    }

    Ok(())
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
