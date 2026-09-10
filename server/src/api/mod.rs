use axum::Router;
use tower_http::trace::TraceLayer;

use crate::state::AppState;

pub mod version;

/// The full `/api` router. One `.nest` per route group; groups land phase by
/// phase (ADR 0033). Anything not nested here is still served by the .NET
/// process via nginx until its phase ships.
pub fn router(state: AppState) -> Router {
    Router::new()
        .nest("/api/version", version::routes())
        .layer(TraceLayer::new_for_http())
        .with_state(state)
}
