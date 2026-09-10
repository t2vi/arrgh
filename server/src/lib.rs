//! *ARRgh server (Rust). See ADR 0033.
//!
//! S0 skeleton: config + tracing + error type + one real endpoint
//! (`GET /api/version`). Every other `/api/*` group arrives one phase at a
//! time (#123–#131) and nginx flips its prefix here once its Hurl +
//! integration tests are green.

pub mod api;
pub mod config;
pub mod error;
pub mod state;

use std::net::SocketAddr;

use tokio::net::TcpListener;
use tracing_subscriber::{fmt, EnvFilter};

use crate::config::Config;
use crate::state::AppState;

/// Initialise tracing from `LOG_LEVEL` (debug|info|warn|error), matching the
/// .NET server's console behaviour. `/api/logs` (S1, #123) will add the
/// in-memory ring-buffer layer this reads from.
pub fn init_tracing() {
    let level = std::env::var("LOG_LEVEL").unwrap_or_else(|_| "info".into());
    let filter = EnvFilter::try_new(format!("arrgh_server={level},tower_http={level},info"))
        .unwrap_or_else(|_| EnvFilter::new("info"));
    // ok() — a second init in tests is not an error worth aborting for
    let _ = fmt().with_env_filter(filter).try_init();
}

/// Build the app and serve until SIGINT/SIGTERM.
pub async fn run() -> anyhow::Result<()> {
    init_tracing();

    let config = Config::from_env()?;
    let addr: SocketAddr = config.bind;
    let state = AppState::new(config);

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
