use anyhow::Result;
use axum::Router;
use arrgh_server::{api, indexer, logging, AppState, Config, SourceMap};
#[allow(unused_imports)]
use arrgh_server::indexer::Source as _;
use chrono::Utc;
use sqlx::sqlite::{SqliteConnectOptions, SqliteJournalMode, SqlitePoolOptions};
use std::collections::HashMap;
use std::str::FromStr;
use std::sync::Arc;
use std::sync::atomic::{AtomicU8, Ordering};
use tokio::sync::{Mutex, RwLock};
use tower_http::{cors::CorsLayer, trace::TraceLayer};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};
use uuid::Uuid;

#[tokio::main]
async fn main() -> Result<()> {
    dotenvy::dotenv().ok();

    let log_buf = logging::new_buffer();
    let log_level_gate = Arc::new(AtomicU8::new(logging::LEVEL_INFO));

    tracing_subscriber::registry()
        .with(tracing_subscriber::EnvFilter::try_from_default_env()
            .unwrap_or_else(|_| "arrgh_server=info,tower_http=warn".into()))
        .with(tracing_subscriber::fmt::layer())
        .with(logging::RingBufferLayer::new(log_buf.clone(), log_level_gate.clone()))
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

    // DB download_dir overrides env var if set — only honour absolute paths;
    // a relative path stored here causes covers to land in the container layer
    // (not the volume) and disappear on every image rebuild.
    if let Ok(Some(dir)) = sqlx::query_scalar::<_, String>("SELECT value FROM server_settings WHERE key = 'download_dir'")
        .fetch_optional(&pool)
        .await
    {
        if std::path::Path::new(&dir).is_absolute() {
            config.download_dir = dir;
        } else {
            tracing::warn!(
                "server_settings.download_dir='{}' is relative — ignored, using env default '{}'",
                dir, config.download_dir
            );
        }
    }
    let config = Arc::new(config);

    // Restore persisted log level for the ring buffer
    if let Ok(Some(lvl)) = sqlx::query_scalar::<_, String>("SELECT value FROM server_settings WHERE key = 'log_level'")
        .fetch_optional(&pool)
        .await
    {
        if let Some(v) = logging::level_from_str(&lvl) {
            log_level_gate.store(v, Ordering::Relaxed);
        }
    }

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
        registry_lock: Arc::new(Mutex::new(())),
        page_cache: Arc::new(Mutex::new(HashMap::new())),
        trending_cache: Arc::new(Mutex::new(None)),
        log_buffer: log_buf,
        log_level: log_level_gate,
        http: reqwest::Client::new(),
        update_cache: api::version::new_cache(),
    };

    indexer::start_scheduler(state.clone());
    arrgh_server::downloader::start_worker(state.clone()).await;
    api::version::start_update_checker(state.clone());

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

/// Insert a plugin URL into external_sources if not already present.
async fn bootstrap_plugin(pool: &sqlx::SqlitePool, base_url: &str) {
    let client = reqwest::Client::new();
    let trimmed = base_url.trim_end_matches('/');

    if let Ok(resp) = client.get(format!("{}/plugins", trimmed)).send().await {
        if resp.status().is_success() {
            if let Ok(arr) = resp.json::<Vec<serde_json::Value>>().await {
                for entry in &arr {
                    if let Some(plugin_id) = entry.get("id").and_then(|v| v.as_str()) {
                        let plugin_url = format!("{}/{}", trimmed, plugin_id);
                        bootstrap_single(pool, &plugin_url, entry).await;
                    }
                }
                return;
            }
        }
    }

    bootstrap_probe(pool, trimmed).await;
}

async fn bootstrap_single(pool: &sqlx::SqlitePool, effective_url: &str, info: &serde_json::Value) {
    let already = sqlx::query_scalar::<_, i64>(
        "SELECT COUNT(*) FROM external_sources WHERE base_url = ?"
    )
    .bind(effective_url)
    .fetch_one(pool)
    .await
    .unwrap_or(0);

    if already > 0 {
        tracing::debug!("PLUGIN_URLS: {} already registered, skipping", effective_url);
        return;
    }

    let name = info.get("name").and_then(|v| v.as_str()).unwrap_or(effective_url).to_string();
    let content_types = info.get("content_types")
        .and_then(|v| v.as_array())
        .map(|arr| arr.iter().filter_map(|v| v.as_str()).collect::<Vec<_>>().join(","))
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| "manga".to_string());
    let default_explicit: i64 = info.get("default_explicit")
        .and_then(|v| v.as_bool())
        .unwrap_or(false) as i64;

    let id = Uuid::new_v4().to_string();
    let now = Utc::now();
    match sqlx::query(
        "INSERT OR IGNORE INTO external_sources
         (id, name, base_url, api_key, content_types, enabled, default_explicit, created_at)
         VALUES (?, ?, ?, NULL, ?, 1, ?, ?)"
    )
    .bind(&id).bind(&name).bind(effective_url).bind(&content_types).bind(default_explicit).bind(now)
    .execute(pool)
    .await
    {
        Ok(_) => tracing::info!("PLUGIN_URLS: registered '{}' ({})", name, effective_url),
        Err(e) => tracing::warn!("PLUGIN_URLS: DB insert failed for {}: {}", effective_url, e),
    }
}

async fn bootstrap_probe(pool: &sqlx::SqlitePool, base_url: &str) {
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
            let default_explicit = src.default_explicit() as i64;
            match sqlx::query(
                "INSERT OR IGNORE INTO external_sources
                 (id, name, base_url, api_key, content_types, enabled, default_explicit, created_at)
                 VALUES (?, ?, ?, NULL, ?, 1, ?, ?)"
            )
            .bind(&id).bind(&name).bind(base_url).bind(&content_types).bind(default_explicit).bind(now)
            .execute(pool)
            .await
            {
                Ok(_) => tracing::info!("PLUGIN_URLS: registered '{}' ({})", name, base_url),
                Err(e) => tracing::warn!("PLUGIN_URLS: DB insert failed for {}: {}", base_url, e),
            }
        }
        Err(e) => tracing::debug!("PLUGIN_URLS: probe failed for {}: {}", base_url, e),
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
