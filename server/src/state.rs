use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use std::time::{Duration, Instant};

use sqlx::sqlite::{SqliteConnectOptions, SqliteJournalMode, SqlitePoolOptions};
use sqlx::SqlitePool;

use crate::config::Config;
use crate::logs::LogBuffer;

/// Connects to the same SQLite file the .NET server's EF migrations own —
/// in production .NET always creates/migrates it before Rust ever gets
/// traffic, so `create_if_missing` is just a defensive no-op there; tests
/// rely on it to spin up an isolated temp-file DB. WAL + a busy timeout
/// match ADR 0033's S0 plan for safe concurrent access from both servers
/// during the strangler-fig.
pub async fn connect_db(database_path: &str) -> anyhow::Result<SqlitePool> {
    let opts = SqliteConnectOptions::new()
        .filename(database_path)
        .create_if_missing(true)
        .journal_mode(SqliteJournalMode::Wal)
        .busy_timeout(Duration::from_millis(5000))
        // S4 (#126): RemoveTitle relies on the schema's ON DELETE CASCADE
        // (title -> chapters/title_sources/title_aliases/sync_log/
        // sync_warnings/user_titles/user_title_settings/read_progress) —
        // SQLite only enforces FKs, cascade included, when this is on.
        .foreign_keys(true);

    Ok(SqlitePoolOptions::new().connect_with(opts).await?)
}

/// Shared, cheaply-cloneable app state handed to every handler.
#[derive(Clone)]
pub struct AppState {
    pub config: Arc<Config>,
    pub update: Arc<UpdateCache>,
    pub trending: Arc<TrendingCache>,
    pub page_cache: Arc<PageCache>,
    pub logs: Arc<LogBuffer>,
    // SqlitePool is internally Arc-backed — cheap to clone as-is.
    pub db: SqlitePool,
    // reqwest::Client is internally Arc-backed (connection pool) — cheap to clone as-is.
    // Used for plugin-host calls (chapter-sync S5, Discover S6, downloader S7, media S8).
    pub http: reqwest::Client,
}

impl AppState {
    pub fn new(config: Config, logs: Arc<LogBuffer>, db: SqlitePool) -> Self {
        Self {
            config: Arc::new(config),
            update: Arc::new(UpdateCache::default()),
            trending: Arc::new(TrendingCache::default()),
            page_cache: Arc::new(PageCache::default()),
            logs,
            db,
            http: reqwest::Client::new(),
        }
    }
}

/// Latest GitHub release, populated by the background update checker.
/// Port of the .NET `UpdateCache` singleton. The poller task itself is
/// wired in a follow-up — for now the cache just stays empty and
/// `GET /api/version` reports no update, which is the correct default.
#[derive(Default)]
pub struct UpdateCache {
    inner: RwLock<Option<Release>>,
}

#[derive(Clone)]
struct Release {
    version: String,
    html_url: String,
}

impl UpdateCache {
    pub fn set(&self, version: impl Into<String>, html_url: impl Into<String>) {
        *self.inner.write().unwrap() = Some(Release {
            version: version.into(),
            html_url: html_url.into(),
        });
    }

    pub fn clear(&self) {
        *self.inner.write().unwrap() = None;
    }

    /// `(latest, release_url)` when a newer version is cached, `(None, None)` otherwise.
    pub fn get_if_newer(&self, current: &str) -> (Option<String>, Option<String>) {
        match &*self.inner.read().unwrap() {
            Some(r) if r.version != current => (Some(r.version.clone()), Some(r.html_url.clone())),
            _ => (None, None),
        }
    }
}

/// Per-lane TTL cache for Discover's trending endpoints (ADR 0032, S6 #128).
/// Port of `TrendingCacheService` — one keyed slot per lane
/// (`manga`/`manhwa`/`manhua`/`adult-manhwa`), 1-hour freshness, stale
/// entries kept (not evicted) so a failed refetch can still serve something.
#[derive(Default)]
pub struct TrendingCache {
    inner: RwLock<HashMap<String, (Instant, Vec<crate::discover::DiscoverResult>)>>,
}

const TRENDING_TTL: Duration = Duration::from_secs(3600);

impl TrendingCache {
    pub fn get_fresh(&self, lane: &str) -> Option<Vec<crate::discover::DiscoverResult>> {
        let guard = self.inner.read().unwrap();
        let (fetched_at, results) = guard.get(lane)?;
        (fetched_at.elapsed() < TRENDING_TTL).then(|| results.clone())
    }

    pub fn get_stale(&self, lane: &str) -> Option<Vec<crate::discover::DiscoverResult>> {
        self.inner.read().unwrap().get(lane).map(|(_, r)| r.clone())
    }

    pub fn set(&self, lane: &str, results: Vec<crate::discover::DiscoverResult>) {
        self.inner
            .write()
            .unwrap()
            .insert(lane.to_string(), (Instant::now(), results));
    }

    /// Backdates an existing entry past the TTL without discarding it — port
    /// of `TrendingCacheService.ExpireForTest`, for exercising the
    /// stale-serves-on-failure path in tests.
    pub fn expire_for_test(&self, lane: &str) {
        if let Some((fetched_at, _)) = self.inner.write().unwrap().get_mut(lane) {
            *fetched_at = Instant::now() - TRENDING_TTL - Duration::from_secs(1);
        }
    }
}

/// Per-chapter cache of resolved page URLs (ADR 0033, S8 #130). Port of
/// `PageCacheService` — 300s TTL, plain miss-on-expiry (unlike
/// `TrendingCache` there's no stale-serve fallback; `ServePage` re-fetches
/// on miss).
type PageUrl = (String, Option<String>);

#[derive(Default)]
pub struct PageCache {
    inner: RwLock<HashMap<String, (Instant, Vec<PageUrl>)>>,
}

const PAGE_CACHE_TTL: Duration = Duration::from_secs(300);

impl PageCache {
    pub fn get(&self, chapter_id: &str) -> Option<Vec<PageUrl>> {
        let guard = self.inner.read().unwrap();
        let (fetched_at, pages) = guard.get(chapter_id)?;
        (fetched_at.elapsed() < PAGE_CACHE_TTL).then(|| pages.clone())
    }

    pub fn set(&self, chapter_id: &str, pages: Vec<PageUrl>) {
        self.inner
            .write()
            .unwrap()
            .insert(chapter_id.to_string(), (Instant::now(), pages));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn page_cache_miss_on_empty() {
        assert!(PageCache::default().get("key").is_none());
    }

    #[test]
    fn page_cache_hit_after_set() {
        let cache = PageCache::default();
        let pages = vec![("https://example.com/1.jpg".to_string(), None)];
        cache.set("key", pages.clone());
        assert_eq!(cache.get("key"), Some(pages));
    }
}
