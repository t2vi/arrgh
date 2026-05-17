use anyhow::Result;
use chrono::Utc;
use sqlx::SqlitePool;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;
use uuid::Uuid;

use crate::AppState;

pub mod external;
pub mod local;
pub mod mangapill;
pub mod source;

pub use source::Source;

pub type SourceRegistry = Arc<HashMap<String, Arc<dyn Source>>>;

/// Rebuild registry merging compiled-in sources with enabled external sources from DB.
pub async fn load_registry(db: &sqlx::SqlitePool) -> SourceRegistry {
    let mut map: HashMap<String, Arc<dyn Source>> = HashMap::new();

    let mp = Arc::new(mangapill::Mangapill);
    map.insert(mp.id().to_string(), mp);

    match sqlx::query!(
        r#"SELECT id as "id!", base_url as "base_url!", api_key FROM external_sources WHERE enabled = 1"#
    )
    .fetch_all(db)
    .await {
        Ok(rows) => {
            for row in rows {
                match external::ExternalSource::probe(row.id, row.base_url, row.api_key).await {
                    Ok(src) => {
                        tracing::info!("loaded external source: {} ({})", src.name(), src.id());
                        map.insert(src.id().to_string(), Arc::new(src));
                    }
                    Err(e) => tracing::warn!("external source probe failed: {}", e),
                }
            }
        }
        Err(e) => tracing::warn!("failed to load external sources from DB: {}", e),
    }

    Arc::new(map)
}

pub fn start_scheduler(state: AppState) {
    tokio::spawn(async move {
        let hours = read_setting_i64(&state.db, "index_interval_hours",
            state.config.index_interval_hours as i64).await as u64;
        let mut interval = tokio::time::interval(Duration::from_secs(hours * 3600));
        loop {
            interval.tick().await;
            tracing::info!("running scheduled library sync");
            if let Err(e) = sync_library(&state).await {
                tracing::error!("sync_library error: {}", e);
            }
        }
    });
}

/// Full library sync: local file scan + remote chapter sync for all manga.
pub async fn sync_library(state: &AppState) -> Result<()> {
    local::scan(&state.db, &state.config.download_dir).await?;
    local::scan_downloads(&state.db, &state.config.download_dir).await?;

    let global_auto_dl = read_setting_bool(&state.db, "auto_download").await;

    let mangas = sqlx::query!(
        r#"SELECT id as "id!", source as "source!", source_id, auto_download
           FROM manga
           WHERE source != 'local' AND source_id IS NOT NULL AND sync_status = 'ready'
             AND EXISTS (SELECT 1 FROM user_manga WHERE manga_id = manga.id)"#
    )
    .fetch_all(&state.db)
    .await?;

    for manga in mangas {
        let source_id = match manga.source_id {
            Some(s) => s,
            None => continue,
        };
        let src = match state.sources.read().await.get(&manga.source).cloned() {
            Some(s) => s,
            None => {
                tracing::warn!("no source impl for '{}', skipping", manga.source);
                continue;
            }
        };

        sqlx::query!("UPDATE manga SET sync_status = 'syncing' WHERE id = ?", manga.id)
            .execute(&state.db)
            .await?;

        let result = src.sync_chapters(&state.db, &manga.id, &source_id).await;
        let status = if result.is_ok() { "ready" } else { "error" };
        let mid = manga.id.as_str();
        sqlx::query!("UPDATE manga SET sync_status = ? WHERE id = ?", status, mid)
            .execute(&state.db)
            .await?;

        if let Err(e) = result {
            tracing::error!("sync error for manga {}: {}", mid, e);
            continue;
        }

        // Update is_explicit from meta tags (sources like Toonily encode explicit genre here)
        if let Ok(meta) = src.fetch_meta(&source_id).await {
            if let Some(tags) = meta.tags {
                let tag_explicit = tags.split(',')
                    .any(|t| t.trim().eq_ignore_ascii_case("adult"));
                if tag_explicit {
                    let _ = sqlx::query!(
                        "UPDATE manga SET is_explicit = 1, tags = COALESCE(tags, ?) WHERE id = ?",
                        tags, mid
                    )
                    .execute(&state.db)
                    .await
                    .map_err(|e| tracing::warn!("is_explicit update error for {}: {}", mid, e));
                }
            }
        }

        let effective = manga.auto_download
            .map(|v| v != 0)
            .unwrap_or(global_auto_dl);

        if effective {
            if let Err(e) = queue_new_chapters(&state.db, mid).await {
                tracing::error!("queue_new_chapters error for {}: {}", mid, e);
            }
        }
    }

    Ok(())
}

async fn queue_new_chapters(db: &SqlitePool, manga_id: &str) -> Result<()> {
    let manga = sqlx::query!("SELECT title FROM manga WHERE id = ?", manga_id)
        .fetch_one(db)
        .await?;

    let chapters = sqlx::query!(
        "SELECT id, number FROM chapters WHERE manga_id = ? AND downloaded = 0 AND source_id IS NOT NULL",
        manga_id
    )
    .fetch_all(db)
    .await?;

    let now = Utc::now();
    for ch in chapters {
        let qid = Uuid::new_v4().to_string();
        sqlx::query!(
            r#"INSERT INTO download_queue (id, chapter_id, manga_title, chapter_num, status, created_at, updated_at)
               VALUES (?, ?, ?, ?, 'pending', ?, ?)
               ON CONFLICT(chapter_id) DO UPDATE SET
                 status = 'pending', error = NULL, updated_at = excluded.updated_at
               WHERE download_queue.status IN ('error', 'cancelled')"#,
            qid, ch.id, manga.title, ch.number, now, now
        )
        .execute(db)
        .await?;
    }

    Ok(())
}

async fn read_setting_bool(db: &SqlitePool, key: &str) -> bool {
    sqlx::query_scalar!("SELECT value FROM server_settings WHERE key = ?", key)
        .fetch_optional(db)
        .await
        .map_err(|e| tracing::warn!("setting read error for '{}': {}", key, e))
        .ok()
        .flatten()
        .map(|v: String| v == "true")
        .unwrap_or(false)
}

async fn read_setting_i64(db: &SqlitePool, key: &str, default: i64) -> i64 {
    sqlx::query_scalar!("SELECT value FROM server_settings WHERE key = ?", key)
        .fetch_optional(db)
        .await
        .map_err(|e| tracing::warn!("setting read error for '{}': {}", key, e))
        .ok()
        .flatten()
        .and_then(|v: String| v.parse().ok())
        .unwrap_or(default)
}
