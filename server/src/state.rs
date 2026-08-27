use std::sync::{Arc, RwLock};

use crate::config::Config;

/// Shared, cheaply-cloneable app state handed to every handler.
#[derive(Clone)]
pub struct AppState {
    pub config: Arc<Config>,
    pub update: Arc<UpdateCache>,
    // S2 (#124) adds `db: sqlx::SqlitePool` here.
}

impl AppState {
    pub fn new(config: Config) -> Self {
        Self {
            config: Arc::new(config),
            update: Arc::new(UpdateCache::default()),
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
