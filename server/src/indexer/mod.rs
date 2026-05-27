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
/// Probes each source's /info to get current default_explicit and content_types,
/// updating the DB record so the stored value stays in sync.
pub async fn load_registry(db: &sqlx::SqlitePool) -> SourceRegistry {
    let mut map: HashMap<String, Arc<dyn Source>> = HashMap::new();

    let rows = match sqlx::query!(
        r#"SELECT id as "id!", name as "name!", base_url as "base_url!",
                  api_key, content_types as "content_types!",
                  default_explicit as "default_explicit!"
           FROM external_sources WHERE enabled = 1"#
    )
    .fetch_all(db)
    .await {
        Ok(r) => r,
        Err(e) => { tracing::warn!("failed to load external sources from DB: {}", e); return Arc::new(map); }
    };

    for row in rows {
        let source_key = row.base_url
            .trim_end_matches('/')
            .rsplit('/')
            .next()
            .unwrap_or(&row.base_url)
            .to_string();

        // Probe /info to get live default_explicit and content_types.
        // If probe fails, fall back to DB values.
        let src = match external::ExternalSource::probe(
            row.id.clone(),
            row.base_url.clone(),
            row.api_key.clone(),
        ).await {
            Ok(probed) => {
                // Update DB with probed values so they stay in sync
                let explicit_i = probed.default_explicit() as i64;
                let ct_str = probed.content_types().join(",");
                let _ = sqlx::query!(
                    "UPDATE external_sources SET default_explicit = ?, content_types = ?, source_key = ? WHERE id = ?",
                    explicit_i, ct_str, source_key, row.id
                )
                .execute(db)
                .await
                .map_err(|e| tracing::warn!("failed to update source metadata for {}: {}", row.id, e));
                probed
            }
            Err(e) => {
                tracing::debug!("probe failed for {} ({}), using DB values: {}", row.name, row.base_url, e);
                let ct: Vec<String> = row.content_types
                    .split(',')
                    .map(|s| s.trim().to_string())
                    .filter(|s| !s.is_empty())
                    .collect();
                let ct = if ct.is_empty() { vec!["manga".to_string()] } else { ct };
                let _ = sqlx::query!(
                    "UPDATE external_sources SET source_key = ? WHERE id = ?",
                    source_key, row.id
                )
                .execute(db)
                .await
                .map_err(|e| tracing::warn!("failed to update source_key for {}: {}", row.id, e));
                external::ExternalSource::new(
                    row.id.clone(), source_key.clone(), row.name, row.base_url, row.api_key, ct,
                    row.default_explicit != 0,
                )
            }
        };

        tracing::info!("loaded external source: {} ({}, explicit={})", src.name(), src.id(), src.default_explicit());
        map.insert(source_key, Arc::new(src));
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

    // Fetch all (title_id, source, source_id) for titles that are ready and in at least one library
    let source_links = sqlx::query!(
        r#"SELECT ts.title_id as "title_id!", ts.source as "source!", ts.source_id as "source_id!",
                  m.auto_download
           FROM title_sources ts
           JOIN titles m ON m.id = ts.title_id
           WHERE m.sync_status = 'ready'
             AND EXISTS (SELECT 1 FROM user_titles WHERE title_id = m.id)"#
    )
    .fetch_all(&state.db)
    .await?;

    // Group source links by title_id
    let mut by_title: HashMap<String, Vec<(String, String, Option<i64>)>> = HashMap::new();
    for row in source_links {
        by_title
            .entry(row.title_id)
            .or_default()
            .push((row.source, row.source_id, row.auto_download));
    }

    for (title_id, links) in by_title {
        let updated = sqlx::query!("UPDATE titles SET sync_status = 'syncing' WHERE id = ?", title_id)
            .execute(&state.db)
            .await?;

        if updated.rows_affected() == 0 {
            tracing::debug!("title {} removed between sync query and sync loop, skipping", title_id);
            continue;
        }

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

            match src.sync_chapters(&state.db, &title_id, source_id).await {
                Ok(_) => { any_ok = true; }
                Err(e) => tracing::error!("sync error for title {} source {}: {}", title_id, source, e),
            }

            // Update is_explicit from meta tags using first responsive source
            if any_ok {
                if let Ok(meta) = src.fetch_meta(source_id).await {
                    if let Some(tags) = meta.tags {
                        let tag_explicit = tags.split(',')
                            .any(|t| t.trim().eq_ignore_ascii_case("adult"));
                        if tag_explicit {
                            let _ = sqlx::query!(
                                "UPDATE titles SET is_explicit = 1, tags = COALESCE(tags, ?) WHERE id = ?",
                                tags, title_id
                            )
                            .execute(&state.db)
                            .await
                            .map_err(|e| tracing::warn!("is_explicit update error for {}: {}", title_id, e));
                        }
                    }
                }
            }
        }

        let status = if any_ok { "ready" } else { "error" };
        sqlx::query!("UPDATE titles SET sync_status = ? WHERE id = ?", status, title_id)
            .execute(&state.db)
            .await?;

        if any_ok {
            let effective = auto_download.map(|v| v != 0).unwrap_or(global_auto_dl);
            if effective {
                if let Err(e) = queue_new_chapters(&state.db, &title_id).await {
                    tracing::error!("queue_new_chapters error for {}: {}", title_id, e);
                }
            }
        }
    }

    Ok(())
}

async fn queue_new_chapters(db: &SqlitePool, title_id: &str) -> Result<()> {
    let title = sqlx::query!("SELECT title FROM titles WHERE id = ?", title_id)
        .fetch_one(db)
        .await?;

    // Queue chapters that have at least one source available for download
    let chapters = sqlx::query!(
        r#"SELECT c.id as "id!", c.number as "number!"
           FROM chapters c
           WHERE c.title_id = ? AND c.downloaded = 0
             AND EXISTS (SELECT 1 FROM chapter_sources WHERE chapter_id = c.id)"#,
        title_id
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
            qid, ch.id, title.title, ch.number, now, now
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
