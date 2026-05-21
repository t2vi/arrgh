use axum::{
    extract::State,
    response::{IntoResponse, Json, Response},
    routing::get,
    Extension, Router,
};
use serde::Serialize;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::Mutex;

use crate::{auth::Claims, AppState};
use super::ApiResult;

const CURRENT: &str = env!("CARGO_PKG_VERSION");
const REPO: &str = "t2vi/arrgh";
const INTERVAL: Duration = Duration::from_secs(3600);

pub struct CachedRelease {
    /// Tag version with 'v' stripped, e.g. "0.0.9"
    version: String,
    html_url: String,
    #[allow(dead_code)]
    fetched_at: Instant,
}

pub type UpdateCache = Arc<Mutex<Option<CachedRelease>>>;

pub fn new_cache() -> UpdateCache {
    Arc::new(Mutex::new(None))
}

#[derive(Serialize)]
pub struct VersionResponse {
    current: &'static str,
    /// Non-null only when check is enabled and a newer version exists.
    latest: Option<String>,
    release_url: Option<String>,
}

pub fn router() -> Router<AppState> {
    Router::new().route("/version", get(get_version))
}

async fn get_version(
    State(state): State<AppState>,
    Extension(_claims): Extension<Claims>,
) -> ApiResult<Response> {
    let cache = state.update_cache.lock().await;
    let (latest, release_url) = match &*cache {
        Some(c) if c.version.as_str() != CURRENT => {
            (Some(c.version.clone()), Some(c.html_url.clone()))
        }
        _ => (None, None),
    };
    Ok(Json(VersionResponse { current: CURRENT, latest, release_url }).into_response())
}

pub fn start_update_checker(state: AppState) {
    tokio::spawn(async move {
        loop {
            let enabled = sqlx::query_scalar::<_, String>(
                "SELECT value FROM server_settings WHERE key = 'check_for_updates'"
            )
            .fetch_optional(&state.db)
            .await
            .ok()
            .flatten()
            .map(|v| v == "true")
            .unwrap_or(false);

            if enabled {
                match fetch_latest(&state).await {
                    Ok(release) => {
                        let mut cache = state.update_cache.lock().await;
                        *cache = Some(release);
                    }
                    Err(e) => tracing::debug!("update check failed: {}", e),
                }
            } else {
                state.update_cache.lock().await.take();
            }

            tokio::time::sleep(INTERVAL).await;
        }
    });
}

async fn fetch_latest(state: &AppState) -> anyhow::Result<CachedRelease> {
    let url = format!("https://api.github.com/repos/{}/releases/latest", REPO);
    let resp = state.http.get(&url)
        .header("User-Agent", "arrgh-server")
        .header("Accept", "application/vnd.github+json")
        .send()
        .await?;

    anyhow::ensure!(resp.status().is_success(), "GitHub API {}", resp.status());

    let json: serde_json::Value = resp.json().await?;
    let tag = json["tag_name"].as_str()
        .ok_or_else(|| anyhow::anyhow!("missing tag_name"))?;
    let html_url = json["html_url"].as_str()
        .ok_or_else(|| anyhow::anyhow!("missing html_url"))?
        .to_string();
    let version = tag.strip_prefix('v').unwrap_or(tag).to_string();

    Ok(CachedRelease { version, html_url, fetched_at: Instant::now() })
}
