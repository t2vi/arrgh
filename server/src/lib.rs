pub mod api;
pub mod auth;
pub mod config;
pub mod db;
pub mod downloader;
pub mod ehentai;
pub mod indexer;
pub mod logging;
pub mod mangaupdates;
pub mod media;

pub use config::Config;

use std::collections::HashMap;
use std::sync::Arc;
use std::time::Instant;
use tokio::sync::{Mutex, RwLock};

pub type SourceMap = Arc<RwLock<HashMap<String, Arc<dyn indexer::Source>>>>;
pub type PageCache = Arc<Mutex<HashMap<String, (Instant, Arc<Vec<indexer::source::PageUrl>>)>>>;

#[derive(Clone)]
pub struct AppState {
    pub db: sqlx::SqlitePool,
    pub config: Arc<Config>,
    pub jwt_secret: Arc<String>,
    pub sources: SourceMap,
    pub registry_lock: Arc<Mutex<()>>,
    pub page_cache: PageCache,
    pub trending_cache: api::discover::TrendingCache,
    pub log_buffer: logging::LogBuffer,
    pub log_level: logging::LevelGate,
    pub http: reqwest::Client,
    pub update_cache: api::version::UpdateCache,
}
