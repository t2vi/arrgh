use axum::Router;
use tower_http::trace::TraceLayer;

use crate::state::AppState;

pub mod auth;
pub mod logs;
pub mod settings;
pub mod sources;
pub mod users;
pub mod version;

/// The full `/api` router. One `.nest` per route group; groups land phase by
/// phase (ADR 0033). Anything not nested here is still served by the .NET
/// process via nginx until its phase ships.
pub fn router(state: AppState) -> Router {
    Router::new()
        .nest("/api/version", version::routes())
        .nest("/api/logs", logs::routes())
        .nest("/api/auth", auth::routes())
        .nest("/api/users", users::routes())
        .nest("/api/settings", settings::routes())
        .nest("/api/sources", sources::routes())
        .layer(TraceLayer::new_for_http())
        .with_state(state)
}
