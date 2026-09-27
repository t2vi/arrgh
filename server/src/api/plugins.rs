//! `/api/plugins` — catalog listing, install/remove (ADR 0033, S9 #131), and
//! in-app update/revert with what's actually loaded (spec 031 phase A). The
//! index is auth-only; everything else is admin-gated. plugin-host does the
//! downloading, checksum check and loading; this validates against the
//! catalog first and relays plugin-host's reason when it refuses.

use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::routing::{get, post};
use axum::{Json, Router};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

use crate::auth::Claims;
use crate::error::{AppError, AppResult};
use crate::state::AppState;
use crate::{plugins, sources};

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/", get(status))
        .route("/index", get(index))
        .route("/install", post(install))
        .route("/{id}", axum::routing::delete(delete_plugin))
        .route("/{id}/update", post(update))
        .route("/{id}/revert", post(revert))
}

fn host_url(state: &AppState, path: &str) -> String {
    format!(
        "{}{path}",
        state.config.plugin_host_url.trim_end_matches('/')
    )
}

async fn catalog(state: &AppState) -> Option<(Vec<plugins::PluginIndexEntry>, &'static str)> {
    plugins::fetch_catalog(
        &state.config.plugin_index_url,
        &state.config.plugin_index_fallback_url,
        &state.http,
    )
    .await
}

async fn index(
    _claims: Claims,
    State(state): State<AppState>,
) -> AppResult<Json<Vec<plugins::PluginIndexEntry>>> {
    let (entries, _) = catalog(&state).await.ok_or(AppError::BadGateway)?;
    Ok(Json(entries))
}

/// What plugin-host reports for one loaded plugin (`GET /plugins`).
#[derive(Deserialize)]
struct HostPlugin {
    id: String,
    name: String,
    #[serde(default)]
    version: Option<String>,
    #[serde(default)]
    origin: Option<String>,
    #[serde(default)]
    has_bundled: bool,
    /// Older hosts (and the e2e fixture) send only this, no `origin`.
    #[serde(default)]
    is_community: bool,
}

#[derive(Deserialize)]
struct HostInfo {
    protocol: u32,
}

#[derive(Serialize)]
struct PluginStatus {
    id: String,
    name: String,
    /// `None` = not loaded, or loaded but reports no version ("unknown").
    loaded_version: Option<String>,
    /// `bundled` / `downloaded`; `None` = not loaded.
    origin: Option<String>,
    has_bundled: bool,
    catalog_version: Option<String>,
    update_available: bool,
    /// Set only when an update is available but can't run.
    blocked_reason: Option<String>,
}

#[derive(Serialize)]
struct StatusResponse {
    /// `live` / `fallback`; `None` when no catalog could be read.
    catalog: Option<&'static str>,
    host_protocol: Option<u32>,
    plugins: Vec<PluginStatus>,
}

async fn host_protocol(state: &AppState) -> Option<u32> {
    let res = state.http.get(host_url(state, "/host")).send().await.ok()?;
    Some(
        res.error_for_status()
            .ok()?
            .json::<HostInfo>()
            .await
            .ok()?
            .protocol,
    )
}

/// Loaded plugins merged with the catalog (FR-003). Read-only: never
/// installs anything (FR-011).
async fn status(claims: Claims, State(state): State<AppState>) -> AppResult<Json<StatusResponse>> {
    claims.require_admin()?;
    let loaded: Vec<HostPlugin> = async {
        let res = state
            .http
            .get(host_url(&state, "/plugins"))
            .send()
            .await
            .ok()?;
        res.error_for_status().ok()?.json().await.ok()
    }
    .await
    .ok_or(AppError::Upstream("plugin host unreachable".into()))?;
    let protocol = host_protocol(&state).await;
    let (entries, from) = match catalog(&state).await {
        Some((e, from)) => (e, Some(from)),
        None => (Vec::new(), None),
    };

    let mut rows: Vec<PluginStatus> = loaded
        .into_iter()
        .map(|p| {
            let entry = entries.iter().find(|e| e.id == p.id);
            let update_available =
                entry.is_some_and(|e| plugins::update_available(p.version.as_deref(), &e.version));
            PluginStatus {
                blocked_reason: entry
                    .filter(|_| update_available)
                    .and_then(|e| plugins::blocked_reason(e, protocol)),
                catalog_version: entry.map(|e| e.version.clone()),
                update_available,
                id: p.id,
                name: p.name,
                loaded_version: p.version,
                origin: p.origin.or_else(|| {
                    let o = if p.is_community {
                        plugins::ORIGIN_DOWNLOADED
                    } else {
                        plugins::ORIGIN_BUNDLED
                    };
                    Some(o.to_string())
                }),
                has_bundled: p.has_bundled,
            }
        })
        .collect();
    let loaded_ids: Vec<String> = rows.iter().map(|r| r.id.clone()).collect();
    for e in entries.iter().filter(|e| !loaded_ids.contains(&e.id)) {
        rows.push(PluginStatus {
            id: e.id.clone(),
            name: e.name.clone(),
            loaded_version: None,
            origin: None,
            has_bundled: false,
            catalog_version: Some(e.version.clone()),
            update_available: false,
            blocked_reason: None,
        });
    }
    Ok(Json(StatusResponse {
        catalog: from,
        host_protocol: protocol,
        plugins: rows,
    }))
}

/// Ask plugin-host to download, verify and load `entry`. Its 4xx reasons
/// (checksum, protocol, load failure) come back as 422, anything else 502.
async fn host_install(state: &AppState, entry: &plugins::PluginIndexEntry) -> AppResult<Value> {
    let res = state
        .http
        .post(host_url(state, "/plugins/install"))
        .json(&json!({
            "id": entry.id,
            "url": entry.download_url,
            "sha256": entry.sha256,
            "protocol": entry.protocol,
        }))
        .send()
        .await
        .map_err(|e| AppError::Upstream(format!("plugin host unreachable: {e}")))?;
    let status = res.status();
    let body: Value = res.json().await.unwrap_or(Value::Null);
    let reason = || {
        body["error"]
            .as_str()
            .unwrap_or("plugin host error")
            .to_string()
    };
    if status.is_success() {
        Ok(body)
    } else if status.is_client_error() {
        Err(AppError::UnprocessableEntity(reason()))
    } else {
        Err(AppError::Upstream(reason()))
    }
}

/// Update a plugin to the catalog's version (FR-005, FR-006, FR-012).
async fn update(
    claims: Claims,
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> AppResult<Json<Value>> {
    claims.require_admin()?;
    let (entries, _) = catalog(&state)
        .await
        .ok_or(AppError::Upstream("plugin catalog unreachable".into()))?;
    let entry = entries
        .into_iter()
        .find(|e| e.id == id)
        .ok_or(AppError::NotFound)?;
    if let Some(reason) = plugins::blocked_reason(&entry, host_protocol(&state).await) {
        return Err(AppError::UnprocessableEntity(reason));
    }
    Ok(Json(host_install(&state, &entry).await?))
}

/// Back to the bundled version (FR-009). 409 when nothing was downloaded.
async fn revert(
    claims: Claims,
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> AppResult<(StatusCode, Json<Value>)> {
    claims.require_admin()?;
    let res = state
        .http
        .delete(host_url(&state, &format!("/plugins/{id}")))
        .send()
        .await
        .map_err(|e| AppError::Upstream(format!("plugin host unreachable: {e}")))?;
    match res.status() {
        StatusCode::OK => Ok((
            StatusCode::OK,
            Json(res.json().await.unwrap_or(Value::Null)),
        )),
        StatusCode::NO_CONTENT => Ok((StatusCode::NO_CONTENT, Json(Value::Null))),
        StatusCode::FORBIDDEN => Err(AppError::Conflict(
            "nothing to revert — already the bundled version".into(),
        )),
        StatusCode::NOT_FOUND => Err(AppError::NotFound),
        s => Err(AppError::Upstream(format!("plugin host answered {s}"))),
    }
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

    let (entries, _) = catalog(&state).await.ok_or(AppError::BadGateway)?;
    let entry = entries
        .into_iter()
        .find(|e| e.id == plugin_id)
        .ok_or(AppError::NotFound)?;

    if entry
        .download_url
        .as_deref()
        .is_none_or(|u| u.trim().is_empty())
    {
        return Err(AppError::UnprocessableEntity(String::new()));
    }
    if entry.sha256.as_deref().is_none_or(|s| s.trim().is_empty()) {
        return Err(AppError::UnprocessableEntity(
            "the catalog entry has no checksum".into(),
        ));
    }

    let effective_url = format!(
        "{}/{}",
        state.config.plugin_host_url.trim_end_matches('/'),
        plugin_id
    );

    // Check duplicate before calling plugin-host (saves a round-trip on conflicts).
    if sources::exists_by_base_url(&state.db, &effective_url).await? {
        return Err(AppError::Conflict(String::new()));
    }

    host_install(&state, &entry).await?;

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
    let _ = state
        .http
        .delete(host_url(&state, &format!("/plugins/{id}")))
        .send()
        .await;

    sources::delete(&state.db, &source.id).await?;
    Ok(StatusCode::NO_CONTENT)
}
