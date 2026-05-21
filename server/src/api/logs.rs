use axum::{
    extract::{Query, State},
    http::StatusCode,
    response::{IntoResponse, Json, Response},
    routing::get,
    Extension, Router,
};
use serde::{Deserialize, Serialize};
use std::sync::atomic::Ordering;

use crate::{auth::Claims, logging, AppState};
use super::ApiResult;

#[derive(Deserialize)]
pub struct LogsQuery {
    #[serde(default = "default_limit")]
    limit: usize,
}
fn default_limit() -> usize { 200 }

#[derive(Deserialize)]
pub struct SetLevelBody {
    level: String,
}

#[derive(Serialize)]
pub struct LogLevelResponse {
    level: String,
}

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/logs", get(get_logs))
        .route("/logs/level", get(get_level).patch(set_level))
}

async fn get_logs(
    State(state): State<AppState>,
    Extension(_claims): Extension<Claims>,
    Query(q): Query<LogsQuery>,
) -> ApiResult<Response> {
    let limit = q.limit.min(500);
    let buf = state.log_buffer.lock().unwrap();
    let skip = buf.len().saturating_sub(limit);
    let entries: Vec<&logging::LogEntry> = buf.iter().skip(skip).collect();
    Ok(Json(entries).into_response())
}

async fn get_level(
    State(state): State<AppState>,
    Extension(_claims): Extension<Claims>,
) -> ApiResult<Response> {
    let v = state.log_level.load(Ordering::Relaxed);
    Ok(Json(LogLevelResponse { level: logging::level_to_str(v).to_string() }).into_response())
}

async fn set_level(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Json(body): Json<SetLevelBody>,
) -> ApiResult<Response> {
    if claims.role != "admin" {
        return Ok(StatusCode::FORBIDDEN.into_response());
    }
    let Some(v) = logging::level_from_str(&body.level) else {
        return Ok(StatusCode::UNPROCESSABLE_ENTITY.into_response());
    };
    state.log_level.store(v, Ordering::Relaxed);
    sqlx::query(
        "INSERT INTO server_settings (key, value) VALUES ('log_level', ?)
         ON CONFLICT(key) DO UPDATE SET value = excluded.value"
    )
    .bind(&body.level)
    .execute(&state.db)
    .await?;
    Ok(StatusCode::NO_CONTENT.into_response())
}
