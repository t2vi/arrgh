use anyhow::Result;
use axum::Router;
use dotenvy::dotenv;
use sqlx::sqlite::SqlitePoolOptions;
use std::sync::Arc;
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

#[derive(Clone)]
pub struct AppState {
    pub db: sqlx::SqlitePool,
    pub config: Arc<Config>,
    pub jwt_secret: Arc<String>,
    pub sources: indexer::SourceRegistry,
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

    let config = Arc::new(Config::from_env()?);

    let pool = SqlitePoolOptions::new()
        .max_connections(5)
        .connect(&config.database_url)
        .await?;

    sqlx::migrate!("./migrations").run(&pool).await?;

    indexer::local::verify_downloads(&pool).await?;

    let jwt_secret = Arc::new(get_or_create_jwt_secret(&pool).await?);

    let sources = indexer::build_registry();
    let state = AppState { db: pool, config: config.clone(), jwt_secret, sources };

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
