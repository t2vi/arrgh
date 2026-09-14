//! `/api/plugins` — port of `Api/Plugins.cs` (ADR 0033, S9 #131). Index is
//! auth-only; install/delete are admin-gated. plugin-host itself is
//! unchanged — this proxies to it exactly as .NET did.

use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::routing::get;
use axum::{Json, Router};
use serde::Deserialize;

use crate::auth::Claims;
use crate::error::{AppError, AppResult};
use crate::state::AppState;
use crate::{plugins, sources};

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/index", get(index))
        .route("/install", axum::routing::post(install))
        .route("/{id}", axum::routing::delete(delete_plugin))
}

async fn index(
    _claims: Claims,
    State(state): State<AppState>,
) -> AppResult<Json<Vec<plugins::PluginIndexEntry>>> {
    let entries = plugins::fetch_index(&state.config.plugin_index_url, &state.http)
        .await
        .ok_or(AppError::BadGateway)?;
    Ok(Json(entries))
}

#[derive(Deserialize, Default)]
struct InstallBody {
    #[serde(default)]
    plugin_id: Option<String>,
}

async fn install(
    claims: Claims,
    State(state): State<AppState>,
    Json(body): Json<InstallBody>,
) -> AppResult<StatusCode> {
    claims.require_admin()?;

    let plugin_id = body.plugin_id.unwrap_or_default();
    if plugin_id.trim().is_empty() {
        return Err(AppError::BadRequest("plugin_id required".into()));
    }

    let entries = plugins::fetch_index(&state.config.plugin_index_url, &state.http)
        .await
        .ok_or(AppError::BadGateway)?;
    let entry = entries
        .into_iter()
        .find(|e| e.id == plugin_id)
        .ok_or(AppError::NotFound)?;

    let Some(download_url) = entry.download_url.filter(|u| !u.trim().is_empty()) else {
        return Err(AppError::UnprocessableEntity(String::new()));
    };

    let effective_url = format!(
        "{}/{}",
        state.config.plugin_host_url.trim_end_matches('/'),
        plugin_id
    );

    // Check duplicate before calling plugin-host (saves a round-trip on conflicts).
    if sources::exists_by_base_url(&state.db, &effective_url).await? {
        return Err(AppError::Conflict(String::new()));
    }

    let install_url = format!(
        "{}/plugins/install",
        state.config.plugin_host_url.trim_end_matches('/')
    );
    let install_ok = state
        .http
        .post(&install_url)
        .json(&serde_json::json!({ "url": download_url }))
        .send()
        .await
        .map(|r| r.status().is_success())
        .unwrap_or(false);
    if !install_ok {
        return Err(AppError::BadGateway);
    }

    sources::insert_community_source(
        &state.db,
        &uuid::Uuid::new_v4().to_string(),
        &plugin_id,
        &entry.name,
        &effective_url,
        &entry.content_types.join(","),
        entry.default_explicit,
    )
    .await?;

    Ok(StatusCode::CREATED)
}

async fn delete_plugin(
    claims: Claims,
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> AppResult<StatusCode> {
    claims.require_admin()?;

    let source = sources::find_by_source_key(&state.db, &id)
        .await?
        .ok_or(AppError::NotFound)?;
    if !source.is_community {
        return Err(AppError::Forbidden);
    }

    // Best-effort — don't fail the request if plugin-host is unreachable.
    let uninstall_url = format!(
        "{}/plugins/{id}",
        state.config.plugin_host_url.trim_end_matches('/')
    );
    let _ = state.http.delete(&uninstall_url).send().await;

    sources::delete(&state.db, &source.id).await?;
    Ok(StatusCode::NO_CONTENT)
}
