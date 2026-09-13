//! `GET /api/logs`, `GET /api/logs/level`, `PATCH /api/logs/level` — port of
//! `Api/Logs.cs` (ADR 0033, S1 #123). No DB: the level gate lives only in
//! memory (the .NET side's `db.ServerSettings` persistence isn't ported —
//! S1 scope is explicitly DB-free).

use axum::extract::{Query, State};
use axum::http::StatusCode;
use axum::routing::get;
use axum::{Json, Router};
use serde::{Deserialize, Serialize};

use crate::auth::Claims;
use crate::error::{AppError, AppResult};
use crate::logs::LogEntry;
use crate::state::AppState;

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/", get(list))
        .route("/level", get(get_level).patch(set_level))
}

#[derive(Deserialize)]
struct ListQuery {
    limit: Option<usize>,
}

async fn list(
    _claims: Claims,
    State(state): State<AppState>,
    Query(q): Query<ListQuery>,
) -> Json<Vec<LogEntry>> {
    Json(state.logs.recent(q.limit.unwrap_or(200)))
}

#[derive(Serialize)]
struct LevelResponse {
    level: String,
}

async fn get_level(_claims: Claims, State(state): State<AppState>) -> Json<LevelResponse> {
    Json(LevelResponse {
        level: state.logs.current_level(),
    })
}

#[derive(Deserialize)]
struct SetLevelBody {
    level: String,
}

async fn set_level(
    claims: Claims,
    State(state): State<AppState>,
    Json(body): Json<SetLevelBody>,
) -> AppResult<StatusCode> {
    claims.require_admin()?;

    state
        .logs
        .set_level(&body.level)
        .ok_or_else(|| AppError::UnprocessableEntity("unknown log level".into()))?;

    Ok(StatusCode::NO_CONTENT)
}
