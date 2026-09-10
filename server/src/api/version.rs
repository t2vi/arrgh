use axum::extract::State;
use axum::routing::get;
use axum::{Json, Router};
use serde::Serialize;

use crate::state::AppState;

/// Compiled-in version — single source of truth (ADR 0033). Bump in
/// `server/Cargo.toml`.
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

#[derive(Serialize)]
struct VersionResponse {
    current: &'static str,
    latest: Option<String>,
    release_url: Option<String>,
}

/// `GET /api/version` — port of the .NET `Version.GetVersion`. Same body
/// shape: `{ current, latest, release_url }`, `latest`/`release_url` null
/// unless the update checker has cached a newer release.
async fn get_version(State(state): State<AppState>) -> Json<VersionResponse> {
    let (latest, release_url) = state.update.get_if_newer(VERSION);
    Json(VersionResponse {
        current: VERSION,
        latest,
        release_url,
    })
}

pub fn routes() -> Router<AppState> {
    Router::new().route("/", get(get_version))
}
