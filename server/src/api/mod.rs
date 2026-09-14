use axum::Router;
use tower_http::trace::TraceLayer;

use crate::state::AppState;

pub mod auth;
pub mod chapters;
pub mod logs;
pub mod progress;
pub mod settings;
pub mod sources;
pub mod titles;
pub mod users;
pub mod version;

/// The full `/api` router. One `.nest` per route group; groups land phase by
/// phase (ADR 0033). Anything not nested here is still served by the .NET
/// process via nginx until its phase ships.
///
/// `titles` + `progress` + `chapters` (S4 #126 / S5 #127) are wired here but
/// **not yet flipped in `docker/nginx.conf`** — see `src/titles.rs`'s module
/// doc for why (Discover re-match isn't ported until S6, and the block
/// includes `queue`/S7 too). The router exists so the Rust integration tests
/// can exercise the real handlers ahead of the nginx flip.
pub fn router(state: AppState) -> Router {
    Router::new()
        .nest("/api/version", version::routes())
        .nest("/api/logs", logs::routes())
        .nest("/api/auth", auth::routes())
        .nest("/api/users", users::routes())
        .nest("/api/settings", settings::routes())
        .nest("/api/sources", sources::routes())
        .nest("/api/titles", titles::routes())
        .nest("/api/progress", progress::routes())
        .nest("/api/chapters", chapters::routes())
        .layer(TraceLayer::new_for_http())
        .with_state(state)
}
