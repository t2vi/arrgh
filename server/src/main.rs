use anyhow::Result;
use axum::Router;
use chrono::Utc;
use dotenvy::dotenv;
use indexer::source::Source;
use sqlx::sqlite::{SqliteConnectOptions, SqliteJournalMode, SqlitePoolOptions};
use std::collections::HashMap;
use std::str::FromStr;
use std::sync::Arc;
use std::time::Instant;
use tokio::sync::{Mutex, RwLock};
use tower_http::{cors::CorsLayer, trace::TraceLayer};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};
use uuid::Uuid;

mod api;
mod auth;
mod config;
mod db;
mod downloader;
mod indexer;
mod media;

pub use config::Config;

pub type SourceMap = Arc<RwLock<HashMap<String, Arc<dyn indexer::Source>>>>;
pub type PageCache = Arc<Mutex<HashMap<String, (Instant, Arc<Vec<indexer::source::PageUrl>>)>>>;

#[derive(Clone)]
pub struct AppState {
    pub db: sqlx::SqlitePool,
    pub config: Arc<Config>,
    pub jwt_secret: Arc<String>,
    pub sources: SourceMap,
    pub page_cache: PageCache,
    pub trending_cache: api::discover::TrendingCache,
}

#[tokio::main]
async fn main() -> Result<()> {
    dotenv().ok();

    tracing_subscriber::registry()
        .with(tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or_else(|_| {
            "arrgh_server=debug,tower_http=debug".into()
        }))
        .with(tracing_subscriber::fmt::layer())
        .init();

    let mut config = Config::from_env()?;

    let connect_opts = SqliteConnectOptions::from_str(&config.database_url)?
        .create_if_missing(true)
        .journal_mode(SqliteJournalMode::Wal)
        .busy_timeout(std::time::Duration::from_secs(30));
    let pool = SqlitePoolOptions::new()
        .max_connections(5)
        .connect_with(connect_opts)
        .await?;

    sqlx::migrate!("./migrations").run(&pool).await?;

    // DB download_dir overrides env var if set
    if let Ok(Some(dir)) = sqlx::query_scalar::<_, String>("SELECT value FROM server_settings WHERE key = 'download_dir'")
        .fetch_optional(&pool)
        .await
    {
        config.download_dir = dir;
    }
    let config = Arc::new(config);

    indexer::local::verify_downloads(&pool).await?;

    let jwt_secret = Arc::new(get_or_create_jwt_secret(&pool).await?);

    // Bootstrap any URLs from PLUGIN_URLS env var into external_sources table.
    // Idempotent — skips URLs already registered. Runs before load_registry.
    if let Ok(urls) = std::env::var("PLUGIN_URLS") {
        for raw in urls.split(',').map(str::trim).filter(|s| !s.is_empty()) {
            bootstrap_plugin(&pool, raw).await;
        }
    }

    let registry = indexer::load_registry(&pool).await;
    let sources: SourceMap = Arc::new(RwLock::new(
        Arc::try_unwrap(registry).unwrap_or_else(|a| (*a).clone())
    ));

    let state = AppState {
        db: pool,
        config: config.clone(),
        jwt_secret,
        sources,
        page_cache: Arc::new(Mutex::new(HashMap::new())),
        trending_cache: Arc::new(Mutex::new(None)),
    };

    indexer::start_scheduler(state.clone());
    downloader::start_worker(state.clone()).await;

    let app = Router::new()
        .nest("/api", api::router(state.clone()))
        .layer(TraceLayer::new_for_http())
        .layer(CorsLayer::permissive())
        .with_state(state);

    let addr = config.bind_addr.as_deref().unwrap_or("127.0.0.1:3000");
    let listener = tokio::net::TcpListener::bind(addr).await?;
    tracing::info!("API listening on http://{}", listener.local_addr()?);
    tracing::info!("Docs at http://{}/api/docs", listener.local_addr()?);
    axum::serve(listener, app).await?;

    Ok(())
}

/// Insert a plugin URL into external_sources if not already present, then probe it.
/// Skips silently if the URL is already registered (idempotent).
async fn bootstrap_plugin(pool: &sqlx::SqlitePool, base_url: &str) {
    let already = sqlx::query_scalar::<_, i64>(
        "SELECT COUNT(*) FROM external_sources WHERE base_url = ?"
    )
    .bind(base_url)
    .fetch_one(pool)
    .await
    .unwrap_or(0);

    if already > 0 {
        tracing::debug!("PLUGIN_URLS: {} already registered, skipping", base_url);
        return;
    }

    match indexer::external::ExternalSource::probe(
        Uuid::new_v4().to_string(),
        base_url.to_string(),
        None,
    )
    .await
    {
        Ok(src) => {
            let id = Uuid::new_v4().to_string();
            let now = Utc::now();
            let content_types = src.content_types().join(",");
            let name = src.name().to_string();
            match sqlx::query(
                "INSERT OR IGNORE INTO external_sources
                 (id, name, base_url, api_key, content_types, enabled, created_at)
                 VALUES (?, ?, ?, NULL, ?, 1, ?)"
            )
            .bind(&id).bind(&name).bind(base_url).bind(&content_types).bind(now)
            .execute(pool)
            .await
            {
                Ok(_) => tracing::info!("PLUGIN_URLS: registered '{}' ({})", name, base_url),
                Err(e) => tracing::warn!("PLUGIN_URLS: DB insert failed for {}: {}", base_url, e),
            }
        }
        Err(e) => tracing::warn!("PLUGIN_URLS: probe failed for {}: {}", base_url, e),
    }
}

async fn get_or_create_jwt_secret(pool: &sqlx::SqlitePool) -> Result<String> {
    if let Ok(s) = std::env::var("JWT_SECRET") {
        return Ok(s);
    }

    let row = sqlx::query!("SELECT value FROM server_settings WHERE key = 'jwt_secret'")
        .fetch_optional(pool)
        .await?;

    if let Some(r) = row {
        return Ok(r.value);
    }

    let secret = format!("{}{}", Uuid::new_v4(), Uuid::new_v4());
    sqlx::query!(
        "INSERT INTO server_settings (key, value) VALUES ('jwt_secret', ?)",
        secret
    )
    .execute(pool)
    .await?;

    tracing::info!("generated new JWT secret and stored in DB");
    Ok(secret)
}
