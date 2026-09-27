//! Scheduled library re-sync + auto-download (GH #200, spec 007 FR-001a).
//!
//! Every `index_interval_hours` ("Sync interval" in Settings, re-read each
//! cycle) re-syncs each library title through the same path as
//! `POST /titles/{id}/sync`, then queues the chapters that sync *newly found*
//! when auto-download applies: the title's own `auto_download` if set, else
//! the global setting. The existing backlog is never auto-queued. The first
//! run waits one interval so a restart doesn't trigger a sync storm.

use std::collections::HashSet;
use std::time::Duration;

use sqlx::{FromRow, SqlitePool};

use crate::{chapters, settings, titles};

pub async fn run_loop(pool: SqlitePool, http: reqwest::Client, plugin_host_url: String) {
    loop {
        tokio::time::sleep(interval(&pool).await).await;
        tick(&pool, &http, &plugin_host_url).await;
    }
}

/// Current sync interval, clamped to the Settings UI range (1–24 h).
pub async fn interval(pool: &SqlitePool) -> Duration {
    let raw = settings::get(pool, settings::INDEX_INTERVAL_HOURS)
        .await
        .ok()
        .flatten();
    let hours = settings::parse_long(raw.as_deref(), settings::DEFAULT_INDEX_INTERVAL_HOURS)
        .clamp(1, settings::MAX_INDEX_INTERVAL_HOURS);
    Duration::from_secs(hours as u64 * 3600)
}

#[derive(FromRow)]
struct LibraryTitle {
    id: String,
    content_type: String,
    auto_download: Option<bool>,
}

/// One scheduler pass over every library title not already syncing.
// ponytail: titles sync one after another; parallelise if big libraries make a pass too slow.
pub async fn tick(pool: &SqlitePool, http: &reqwest::Client, plugin_host_url: &str) {
    let global_auto = settings::get(pool, settings::AUTO_DOWNLOAD)
        .await
        .ok()
        .flatten();
    let global_auto = settings::parse_bool(global_auto.as_deref(), settings::DEFAULT_AUTO_DOWNLOAD);

    let library: Vec<LibraryTitle> = match sqlx::query_as(
        "SELECT id, content_type, auto_download FROM titles t \
         WHERE sync_status <> ? \
         AND EXISTS(SELECT 1 FROM user_titles ut WHERE ut.title_id = t.id)",
    )
    .bind(titles::SYNC_SYNCING)
    .fetch_all(pool)
    .await
    {
        Ok(rows) => rows,
        Err(e) => {
            tracing::warn!(error = ?e, "scheduled sync: listing library failed");
            return;
        }
    };

    for t in library {
        let auto = t.auto_download.unwrap_or(global_auto);
        if let Err(e) = sync_one(pool, http, plugin_host_url, &t, auto).await {
            tracing::warn!(error = ?e, title_id = %t.id, "scheduled sync failed");
        }
    }
}

async fn chapter_ids(pool: &SqlitePool, title_id: &str) -> sqlx::Result<Vec<String>> {
    sqlx::query_scalar("SELECT id FROM chapters WHERE title_id = ?")
        .bind(title_id)
        .fetch_all(pool)
        .await
}

async fn sync_one(
    pool: &SqlitePool,
    http: &reqwest::Client,
    plugin_host_url: &str,
    t: &LibraryTitle,
    auto_download: bool,
) -> anyhow::Result<()> {
    let links = titles::title_source_links(pool, &t.id).await?;
    if links.is_empty() {
        return Ok(());
    }
    let before: HashSet<String> = chapter_ids(pool, &t.id).await?.into_iter().collect();

    titles::update_sync_status(pool, &t.id, titles::SYNC_SYNCING).await?;
    titles::clear_sync_log(pool, &t.id).await?;
    titles::append_sync_log(pool, &t.id, "Scheduled sync").await;
    titles::run_sync(pool, http, plugin_host_url, &t.id, &t.content_type, &links).await;

    if !auto_download {
        return Ok(());
    }
    for id in chapter_ids(pool, &t.id).await? {
        if before.contains(&id) {
            continue;
        }
        // The title is already in a library, so the explicit gate is passed;
        // queued_by NULL marks a system (scheduler) entry.
        if let Some(c) = chapters::queue_candidate(pool, &id, true).await? {
            chapters::queue_download(pool, &c.id, &c.manga_title, c.number, None).await?;
        }
    }
    Ok(())
}
