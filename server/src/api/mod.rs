use axum::Router;
use tower_http::trace::TraceLayer;

use crate::state::AppState;

pub mod auth;
pub mod chapters;
pub mod discover;
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
/// doc for why (the ADR's titles/chapters/progress/queue block waits on
/// S7). `discover` (S6 #128) is the exception — it's its own self-contained
/// gate per the ADR and flips as soon as it's green.
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
        .nest("/api/discover", discover::routes())
        .layer(TraceLayer::new_for_http())
        .with_state(state)
}
