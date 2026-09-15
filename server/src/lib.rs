//! *ARRgh server (Rust). See ADR 0033. Sole backend as of S10 (#132) —
//! every `/api/*` group is here, `server/migrations/` owns the schema, and
//! this module seeds the bundled sources on a fresh install.

pub mod api;
pub mod auth;
pub mod chapters;
pub mod config;
pub mod discover;
pub mod downloader;
pub mod error;
pub mod logs;
pub mod media;
pub mod metadata;
pub mod plugins;
pub mod progress;
pub mod queue;
pub mod settings;
pub mod sources;
pub mod state;
pub mod titles;
pub mod update_checker;
pub mod users;

use std::net::SocketAddr;
use std::sync::Arc;

use tokio::net::TcpListener;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;
use tracing_subscriber::{fmt, EnvFilter, Layer};

use crate::config::Config;
use crate::logs::{LogBuffer, LogBufferLayer};
use crate::state::{connect_db, AppState};

/// Initialise tracing: console output gated by `LOG_LEVEL` (fixed at boot,
/// matching the .NET server's console behaviour) plus the `/api/logs` ring
/// buffer, whose gate is independently adjustable at runtime.
pub fn init_tracing(level: &str, buffer: Arc<LogBuffer>) {
    let console_filter =
        EnvFilter::try_new(format!("arrgh_server={level},tower_http={level},info"))
            .unwrap_or_else(|_| EnvFilter::new("info"));

    let registry = tracing_subscriber::registry()
        .with(fmt::layer().with_filter(console_filter))
        .with(LogBufferLayer::new(buffer));

    // ok() — a second init in tests is not an error worth aborting for
    let _ = registry.try_init();
}

/// Build the app and serve until SIGINT/SIGTERM.
pub async fn run() -> anyhow::Result<()> {
    let config = Config::from_env()?;
    let level = std::env::var("LOG_LEVEL").unwrap_or_else(|_| "info".into());
    let log_buffer = LogBuffer::new(&level);
    init_tracing(&level, log_buffer.clone());

    let addr: SocketAddr = config.bind;
    let db = connect_db(&config.database_path).await?;
    if config.seed_default_sources {
        sources::seed_defaults_if_empty(&db, &config.plugin_host_url).await?;
    }
    let state = AppState::new(config, log_buffer, db);

    tokio::spawn(downloader::run_loop(
        state.db.clone(),
        state.http.clone(),
        state.config.plugin_host_url.clone(),
        state.config.download_dir.clone(),
    ));
    tokio::spawn(update_checker::run_loop(
        state.db.clone(),
        state.http.clone(),
        state.update.clone(),
    ));

    let listener = TcpListener::bind(addr).await?;
    tracing::info!(%addr, "arrgh-server listening");

    axum::serve(listener, api::router(state))
        .with_graceful_shutdown(shutdown_signal())
        .await?;
    Ok(())
}

async fn shutdown_signal() {
    use tokio::signal;
    let ctrl_c = async {
        signal::ctrl_c().await.ok();
    };
    #[cfg(unix)]
    let term = async {
        signal::unix::signal(signal::unix::SignalKind::terminate())
            .expect("install SIGTERM handler")
            .recv()
            .await;
    };
    #[cfg(not(unix))]
    let term = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => {},
        _ = term => {},
    }
    tracing::info!("shutdown signal received");
}
