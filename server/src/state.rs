use std::sync::{Arc, RwLock};
use std::time::Duration;

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
    pub logs: Arc<LogBuffer>,
    // SqlitePool is internally Arc-backed — cheap to clone as-is.
    pub db: SqlitePool,
}

impl AppState {
    pub fn new(config: Config, logs: Arc<LogBuffer>, db: SqlitePool) -> Self {
        Self {
            config: Arc::new(config),
            update: Arc::new(UpdateCache::default()),
            logs,
            db,
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
