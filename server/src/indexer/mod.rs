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
pub mod source;

pub use source::Source;

pub type SourceRegistry = Arc<HashMap<String, Arc<dyn Source>>>;

/// Rebuild registry merging compiled-in sources with enabled external sources from DB.
pub async fn load_registry(db: &sqlx::SqlitePool) -> SourceRegistry {
    let mut map: HashMap<String, Arc<dyn Source>> = HashMap::new();

    match sqlx::query!(
        r#"SELECT id as "id!", name as "name!", base_url as "base_url!",
                  api_key, content_types as "content_types!"
           FROM external_sources WHERE enabled = 1"#
    )
    .fetch_all(db)
    .await {
        Ok(rows) => {
            for row in rows {
                let source_key = row.base_url
                    .trim_end_matches('/')
                    .rsplit('/')
                    .next()
                    .unwrap_or(&row.base_url)
                    .to_string();
                let ct: Vec<String> = row.content_types
                    .split(',')
                    .map(|s| s.trim().to_string())
                    .filter(|s| !s.is_empty())
                    .collect();
                let ct = if ct.is_empty() { vec!["manga".to_string()] } else { ct };
                let src = external::ExternalSource::new(
                    row.id.clone(), source_key.clone(), row.name, row.base_url, row.api_key, ct, false,
                );

                // Persist source_key so downloader can join on it for priority ordering
                let _ = sqlx::query!(
                    "UPDATE external_sources SET source_key = ? WHERE id = ?",
                    source_key, row.id
                )
                .execute(db)
                .await
                .map_err(|e| tracing::warn!("failed to update source_key for {}: {}", row.id, e));

                tracing::info!("loaded external source: {} ({})", src.name(), src.id());
                map.insert(source_key, Arc::new(src));
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

    // Fetch all (manga_id, source, source_id) for manga that are ready and in at least one library
    let source_links = sqlx::query!(
        r#"SELECT ms.manga_id as "manga_id!", ms.source as "source!", ms.source_id as "source_id!",
                  m.auto_download
           FROM manga_sources ms
           JOIN manga m ON m.id = ms.manga_id
           WHERE m.sync_status = 'ready'
             AND EXISTS (SELECT 1 FROM user_manga WHERE manga_id = m.id)"#
    )
    .fetch_all(&state.db)
    .await?;

    // Group source links by manga_id
    let mut by_manga: HashMap<String, Vec<(String, String, Option<i64>)>> = HashMap::new();
    for row in source_links {
        by_manga
            .entry(row.manga_id)
            .or_default()
            .push((row.source, row.source_id, row.auto_download));
    }

    for (manga_id, links) in by_manga {
        sqlx::query!("UPDATE manga SET sync_status = 'syncing' WHERE id = ?", manga_id)
            .execute(&state.db)
            .await?;

        let auto_download = links.first().and_then(|(_, _, v)| *v);
        let mut any_ok = false;

        for (source, source_id, _) in &links {
            let src = match state.sources.read().await.get(source).cloned() {
                Some(s) => s,
                None => {
                    tracing::warn!("no source impl for '{}', skipping", source);
                    continue;
                }
            };

            match src.sync_chapters(&state.db, &manga_id, source_id).await {
                Ok(_) => { any_ok = true; }
                Err(e) => tracing::error!("sync error for manga {} source {}: {}", manga_id, source, e),
            }

            // Update is_explicit from meta tags using first responsive source
            if any_ok {
                if let Ok(meta) = src.fetch_meta(source_id).await {
                    if let Some(tags) = meta.tags {
                        let tag_explicit = tags.split(',')
                            .any(|t| t.trim().eq_ignore_ascii_case("adult"));
                        if tag_explicit {
                            let _ = sqlx::query!(
                                "UPDATE manga SET is_explicit = 1, tags = COALESCE(tags, ?) WHERE id = ?",
                                tags, manga_id
                            )
                            .execute(&state.db)
                            .await
                            .map_err(|e| tracing::warn!("is_explicit update error for {}: {}", manga_id, e));
                        }
                    }
                }
            }
        }

        let status = if any_ok { "ready" } else { "error" };
        sqlx::query!("UPDATE manga SET sync_status = ? WHERE id = ?", status, manga_id)
            .execute(&state.db)
            .await?;

        if any_ok {
            let effective = auto_download.map(|v| v != 0).unwrap_or(global_auto_dl);
            if effective {
                if let Err(e) = queue_new_chapters(&state.db, &manga_id).await {
                    tracing::error!("queue_new_chapters error for {}: {}", manga_id, e);
                }
            }
        }
    }

    Ok(())
}

async fn queue_new_chapters(db: &SqlitePool, manga_id: &str) -> Result<()> {
    let manga = sqlx::query!("SELECT title FROM manga WHERE id = ?", manga_id)
        .fetch_one(db)
        .await?;

    // Queue chapters that have at least one source available for download
    let chapters = sqlx::query!(
        r#"SELECT c.id as "id!", c.number as "number!"
           FROM chapters c
           WHERE c.manga_id = ? AND c.downloaded = 0
             AND EXISTS (SELECT 1 FROM chapter_sources WHERE chapter_id = c.id)"#,
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
