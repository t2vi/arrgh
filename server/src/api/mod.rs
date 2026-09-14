use axum::Router;
use tower_http::trace::TraceLayer;

use crate::state::AppState;

pub mod auth;
pub mod chapters;
pub mod discover;
pub mod logs;
pub mod media;
pub mod plugins;
pub mod progress;
pub mod queue;
pub mod settings;
pub mod sources;
pub mod titles;
pub mod users;
pub mod version;

/// The full `/api` router. One `.nest` per route group; groups land phase by
/// phase (ADR 0033). Anything not nested here is still served by the .NET
/// process via nginx until its phase ships.
///
/// `titles`/`progress`/`chapters`/`queue` (S4 #126, S5 #127, S7 #129) flip
/// together in `docker/nginx.conf` now that all four are green — see
/// `src/titles.rs`'s module doc. `discover` (S6 #128), `media` (S8 #130)
/// and `plugins` (S9 #131) are each their own self-contained gate.
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
        .nest("/api/queue", queue::routes())
        .nest("/api/media", media::routes())
        .nest("/api/plugins", plugins::routes())
        .layer(TraceLayer::new_for_http())
        .with_state(state)
}
