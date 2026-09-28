use axum::Router;
use tower_http::trace::{DefaultMakeSpan, TraceLayer};

use crate::state::AppState;

pub mod auth;
pub mod backup;
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

/// The full `/api` router — every route group is here as of S10 (#132);
/// `docker/nginx.conf` proxies all of `/api/` to this server unconditionally
/// (the .NET process this used to share duty with is gone).
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
        .nest("/api/backups", backup::routes())
        // INFO-level span so failure lines (`response failed … 502`) carry the
        // method + URI at the default log level — without it they're undiagnosable.
        .layer(
            TraceLayer::new_for_http()
                .make_span_with(DefaultMakeSpan::new().level(tracing::Level::INFO)),
        )
        .with_state(state)
}
