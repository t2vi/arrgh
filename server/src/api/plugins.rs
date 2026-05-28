use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::{IntoResponse, Json, Response},
    routing::{delete, get, post},
    Extension, Router,
};
use chrono::Utc;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::{auth::Claims, AppState};
use super::ApiResult;

// ── Plugin index types ────────────────────────────────────────────────────────

#[derive(Serialize, Deserialize, Clone)]
pub struct PluginIndexEntry {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    pub version: String,
    pub download_url: Option<String>,
    pub bundled: Option<bool>,
    pub default_explicit: bool,
    pub content_types: Vec<String>,
}

#[derive(Deserialize)]
pub struct InstallBody {
    pub plugin_id: String,
}

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/plugins/index", get(get_index))
        .route("/plugins/install", post(install_plugin))
        .route("/plugins/{id}", delete(delete_plugin))
}

fn admin_only(claims: &Claims) -> Option<Response> {
    if claims.role != "admin" {
        Some(StatusCode::FORBIDDEN.into_response())
    } else {
        None
    }
}

async fn fetch_index(url: &str) -> anyhow::Result<Vec<PluginIndexEntry>> {
    if let Some(path) = url.strip_prefix("file://") {
        let text = tokio::fs::read_to_string(path).await?;
        return Ok(serde_json::from_str(&text)?);
    }
    let entries: Vec<PluginIndexEntry> = Client::new()
        .get(url)
        .send()
        .await?
        .error_for_status()?
        .json()
        .await?;
    Ok(entries)
}

async fn get_index(
    State(state): State<AppState>,
    Extension(_claims): Extension<Claims>,
) -> ApiResult<Response> {
    let url = &state.config.plugin_index_url;
    match fetch_index(url).await {
        Ok(entries) => Ok(Json(entries).into_response()),
        Err(e) => {
            tracing::warn!("plugin index fetch failed ({}): {}", url, e);
            Ok(StatusCode::BAD_GATEWAY.into_response())
        }
    }
}

async fn install_plugin(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Json(body): Json<InstallBody>,
) -> ApiResult<Response> {
    if let Some(r) = admin_only(&claims) { return Ok(r); }

    // Look up plugin in index
    let index = match fetch_index(&state.config.plugin_index_url).await {
        Ok(e) => e,
        Err(e) => {
            tracing::warn!("plugin index fetch failed: {}", e);
            return Ok(StatusCode::BAD_GATEWAY.into_response());
        }
    };

    let entry = match index.into_iter().find(|e| e.id == body.plugin_id) {
        Some(e) => e,
        None => return Ok(StatusCode::NOT_FOUND.into_response()),
    };

    let download_url = match entry.download_url {
        Some(ref u) if !u.is_empty() => u.clone(),
        _ => {
            tracing::warn!("plugin {} has no download_url", body.plugin_id);
            return Ok(StatusCode::UNPROCESSABLE_ENTITY.into_response());
        }
    };

    // Tell plugin-host to download + load the bundle
    let install_url = format!("{}/plugins/install", state.config.plugin_host_url);
    let client = Client::new();
    let resp = client.post(&install_url)
        .json(&serde_json::json!({ "url": download_url }))
        .send()
        .await;

    match resp {
        Ok(r) if r.status().is_success() => {}
        Ok(r) => {
            tracing::warn!("plugin-host install returned {}", r.status());
            return Ok(StatusCode::BAD_GATEWAY.into_response());
        }
        Err(e) => {
            tracing::warn!("plugin-host install request failed: {}", e);
            return Ok(StatusCode::BAD_GATEWAY.into_response());
        }
    }

    // Register the source in external_sources
    let effective_url = format!("{}/{}", state.config.plugin_host_url.trim_end_matches('/'), body.plugin_id);

    let already = sqlx::query_scalar::<_, i64>(
        "SELECT COUNT(*) FROM external_sources WHERE base_url = ?"
    )
    .bind(&effective_url)
    .fetch_one(&state.db)
    .await
    .unwrap_or(0);

    if already > 0 {
        return Ok(StatusCode::CONFLICT.into_response());
    }

    let db_id = Uuid::new_v4().to_string();
    let now = Utc::now();
    let content_types = entry.content_types.join(",");
    let default_explicit = entry.default_explicit as i64;
    sqlx::query!(
        "INSERT INTO external_sources (id, name, base_url, api_key, content_types, enabled, default_explicit, is_community, created_at)
         VALUES (?, ?, ?, NULL, ?, 1, ?, 1, ?)",
        db_id, entry.name, effective_url, content_types, default_explicit, now
    )
    .execute(&state.db)
    .await?;

    super::sources::reload_registry(&state).await?;

    Ok(StatusCode::CREATED.into_response())
}

async fn delete_plugin(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Path(plugin_id): Path<String>,
) -> ApiResult<Response> {
    if let Some(r) = admin_only(&claims) { return Ok(r); }

    let effective_url = format!("{}/{}", state.config.plugin_host_url.trim_end_matches('/'), plugin_id);

    // Only community plugins can be deleted
    let row = sqlx::query!(
        "SELECT id, is_community FROM external_sources WHERE base_url = ?",
        effective_url
    )
    .fetch_optional(&state.db)
    .await?;

    match row {
        None => return Ok(StatusCode::NOT_FOUND.into_response()),
        Some(ref r) if r.is_community == 0 => return Ok(StatusCode::FORBIDDEN.into_response()),
        _ => {}
    }

    // Tell plugin-host to unload + delete
    let unload_url = format!("{}/plugins/{}", state.config.plugin_host_url, plugin_id);
    let client = Client::new();
    if let Err(e) = client.delete(&unload_url).send().await {
        tracing::warn!("plugin-host delete request failed: {}", e);
    }

    // Remove from DB
    if let Some(r) = row {
        sqlx::query!("DELETE FROM external_sources WHERE id = ?", r.id)
            .execute(&state.db)
            .await?;
    }

    super::sources::reload_registry(&state).await?;

    Ok(StatusCode::NO_CONTENT.into_response())
}
