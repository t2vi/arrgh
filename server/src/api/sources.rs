use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::{IntoResponse, Json, Response},
    routing::{get, patch},
    Extension, Router,
};
use chrono::Utc;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use uuid::Uuid;

use crate::{auth::Claims, indexer::{external::ExternalSource, source::Source}, AppState};
use super::ApiResult;

#[derive(Serialize)]
pub struct SourceRow {
    pub id: String,
    pub name: String,
    pub base_url: String,
    pub has_api_key: bool,
    pub content_types: Vec<String>,
    pub enabled: bool,
}

#[derive(Deserialize)]
pub struct AddSourceBody {
    pub base_url: String,
    pub api_key: Option<String>,
}

#[derive(Deserialize)]
pub struct PatchSourceBody {
    pub enabled: bool,
}

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/sources", get(list_sources).post(add_source))
        .route("/sources/{id}", patch(patch_source).delete(delete_source))
}

fn admin_only(claims: &Claims) -> Option<Response> {
    if claims.role != "admin" {
        Some(StatusCode::FORBIDDEN.into_response())
    } else {
        None
    }
}

async fn reload_registry(state: &AppState) -> ApiResult<()> {
    let registry = crate::indexer::load_registry(&state.db).await;
    let map = Arc::try_unwrap(registry).unwrap_or_else(|a| (*a).clone());
    *state.sources.write().await = map;
    Ok(())
}

async fn list_sources(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
) -> ApiResult<Response> {
    if let Some(r) = admin_only(&claims) { return Ok(r); }

    let rows = sqlx::query!(
        r#"SELECT id as "id!", name as "name!", base_url as "base_url!",
                  api_key, content_types as "content_types!", enabled as "enabled!"
           FROM external_sources ORDER BY created_at"#
    )
    .fetch_all(&state.db)
    .await?;

    let out: Vec<SourceRow> = rows.into_iter().map(|r| {
        let content_types = r.content_types.split(',')
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect();
        SourceRow {
            id: r.id,
            name: r.name,
            base_url: r.base_url,
            has_api_key: r.api_key.is_some(),
            content_types,
            enabled: r.enabled != 0,
        }
    }).collect();

    Ok(Json(out).into_response())
}

async fn add_source(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Json(body): Json<AddSourceBody>,
) -> ApiResult<Response> {
    if let Some(r) = admin_only(&claims) { return Ok(r); }

    let probe = match ExternalSource::probe(
        String::new(),
        body.base_url.clone(),
        body.api_key.clone(),
    ).await {
        Ok(s) => s,
        Err(e) => {
            tracing::warn!("source probe failed ({}): {}", body.base_url, e);
            return Ok(StatusCode::BAD_GATEWAY.into_response());
        }
    };

    let db_id = Uuid::new_v4().to_string();
    let now = Utc::now();
    let content_types = probe.content_types().join(",");
    let name = probe.name().to_string();
    sqlx::query!(
        "INSERT INTO external_sources (id, name, base_url, api_key, content_types, enabled, created_at)
         VALUES (?, ?, ?, ?, ?, 1, ?)",
        db_id, name, body.base_url, body.api_key, content_types, now
    )
    .execute(&state.db)
    .await?;

    reload_registry(&state).await?;

    Ok(StatusCode::CREATED.into_response())
}

async fn patch_source(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Path(id): Path<String>,
    Json(body): Json<PatchSourceBody>,
) -> ApiResult<Response> {
    if let Some(r) = admin_only(&claims) { return Ok(r); }

    let enabled_i = body.enabled as i64;
    let affected = sqlx::query!(
        "UPDATE external_sources SET enabled = ? WHERE id = ?",
        enabled_i, id
    )
    .execute(&state.db)
    .await?
    .rows_affected();

    if affected == 0 {
        return Ok(StatusCode::NOT_FOUND.into_response());
    }

    reload_registry(&state).await?;

    Ok(StatusCode::NO_CONTENT.into_response())
}

async fn delete_source(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Path(id): Path<String>,
) -> ApiResult<Response> {
    if let Some(r) = admin_only(&claims) { return Ok(r); }

    let affected = sqlx::query!("DELETE FROM external_sources WHERE id = ?", id)
        .execute(&state.db)
        .await?
        .rows_affected();

    if affected == 0 {
        return Ok(StatusCode::NOT_FOUND.into_response());
    }

    reload_registry(&state).await?;

    Ok(StatusCode::NO_CONTENT.into_response())
}
