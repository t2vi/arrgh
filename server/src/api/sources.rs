//! `/api/sources` — port of `Api/Sources.cs` (ADR 0033, S3 #125). List is
//! auth-only; add/patch/delete are admin-gated. `POST` stays a 502 stub
//! until the plugin system is ported (ADR 0030) — matches .NET exactly.

use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::routing::get;
use axum::{Json, Router};
use serde::{Deserialize, Serialize};

use crate::auth::Claims;
use crate::error::{AppError, AppResult};
use crate::sources;
use crate::state::AppState;

pub fn routes() -> Router<AppState> {
    Router::new().route("/", get(list).post(add)).route(
        "/{id}",
        axum::routing::patch(patch_source).delete(delete_source),
    )
}

#[derive(Serialize)]
struct SourceRowDto {
    id: String,
    name: String,
    base_url: String,
    has_api_key: bool,
    content_types: Vec<String>,
    enabled: bool,
    is_community: bool,
    priority: i64,
}

impl From<sources::SourceRow> for SourceRowDto {
    fn from(s: sources::SourceRow) -> Self {
        Self {
            id: s.id,
            name: s.name,
            base_url: s.base_url,
            has_api_key: s.api_key.is_some(),
            content_types: s
                .content_types
                .split(',')
                .map(str::trim)
                .filter(|s| !s.is_empty())
                .map(String::from)
                .collect(),
            enabled: s.enabled,
            is_community: s.is_community,
            priority: s.priority,
        }
    }
}

async fn list(
    _claims: Claims,
    State(state): State<AppState>,
) -> AppResult<Json<Vec<SourceRowDto>>> {
    let rows = sources::list_ordered_by_created_at(&state.db).await?;
    Ok(Json(rows.into_iter().map(SourceRowDto::from).collect()))
}

#[derive(Deserialize)]
struct AddSourceBody {
    #[allow(dead_code)]
    base_url: String,
    #[allow(dead_code)]
    api_key: Option<String>,
}

/// Stub — must probe the plugin host to get name/content_types/default_explicit.
/// Wired when the plugin system is ported (ADR 0030).
async fn add(claims: Claims, Json(_body): Json<AddSourceBody>) -> AppResult<StatusCode> {
    claims.require_admin()?;
    Ok(StatusCode::BAD_GATEWAY)
}

#[derive(Deserialize)]
struct PatchSourceBody {
    #[serde(default)]
    enabled: bool,
    priority: Option<i64>,
}

async fn patch_source(
    claims: Claims,
    State(state): State<AppState>,
    Path(id): Path<String>,
    Json(body): Json<PatchSourceBody>,
) -> AppResult<StatusCode> {
    claims.require_admin()?;
    sources::find_by_id(&state.db, &id)
        .await?
        .ok_or(AppError::NotFound)?;

    sources::update_enabled(&state.db, &id, body.enabled).await?;
    if let Some(priority) = body.priority {
        sources::update_priority(&state.db, &id, priority).await?;
    }

    Ok(StatusCode::NO_CONTENT)
}

async fn delete_source(
    claims: Claims,
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> AppResult<StatusCode> {
    claims.require_admin()?;
    if !sources::delete(&state.db, &id).await? {
        return Err(AppError::NotFound);
    }
    Ok(StatusCode::NO_CONTENT)
}
