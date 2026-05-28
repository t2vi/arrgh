use axum::{
    extract::State,
    http::{header, Request, StatusCode},
    middleware::{self, Next},
    response::{IntoResponse, Response},
    Router,
};

use crate::{auth, AppState};

pub struct AppError(pub anyhow::Error);
pub type ApiResult<T> = Result<T, AppError>;

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        tracing::error!("internal error: {:?}", self.0);
        StatusCode::INTERNAL_SERVER_ERROR.into_response()
    }
}

impl<E: Into<anyhow::Error>> From<E> for AppError {
    fn from(e: E) -> Self { AppError(e.into()) }
}

mod auth_api;
mod chapters;
pub mod discover;
mod docs;
mod logs;
mod titles;
mod media;
mod plugins;
mod progress;
mod queue;
pub mod settings;
mod sources;
pub mod version;

pub(super) async fn append_sync_log(db: &sqlx::SqlitePool, title_id: &str, message: &str) {
    let id = uuid::Uuid::new_v4().to_string();
    let now = chrono::Utc::now().to_rfc3339();
    let _ = sqlx::query!(
        "INSERT INTO sync_log (id, title_id, message, created_at) VALUES (?, ?, ?, ?)",
        id, title_id, message, now
    )
    .execute(db)
    .await;
}

pub fn router(state: AppState) -> Router<AppState> {
    let protected = Router::new()
        .merge(titles::router())
        .merge(chapters::router())
        .merge(progress::router())
        .merge(discover::router())
        .merge(queue::router())
        .merge(auth_api::protected_router())
        .merge(settings::router())
        .merge(sources::router())
        .merge(plugins::router())
        .merge(logs::router())
        .route_layer(middleware::from_fn_with_state(state, require_auth));

    Router::new()
        .merge(auth_api::public_router())
        .merge(media::router())   // img tags can't send auth headers
        .merge(docs::router())
        .merge(version::router())   // public — health checks + login page version display
        .merge(protected)
}

async fn require_auth(
    State(state): State<AppState>,
    mut req: Request<axum::body::Body>,
    next: Next,
) -> Result<Response, StatusCode> {
    let token = req
        .headers()
        .get(header::AUTHORIZATION)
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.strip_prefix("Bearer "))
        .ok_or(StatusCode::UNAUTHORIZED)?;

    let claims = auth::validate_token(token, &state.jwt_secret)
        .map_err(|_| StatusCode::UNAUTHORIZED)?;

    req.extensions_mut().insert(claims);

    Ok(next.run(req).await)
}
