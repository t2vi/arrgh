use axum::{extract::State, http::StatusCode, response::{IntoResponse, Json, Response}, routing::get, Router};
use serde::{Deserialize, Serialize};

use crate::AppState;
use super::ApiResult;

#[derive(Serialize)]
pub struct AppSettings {
    pub download_workers: i64,
    pub index_interval_hours: i64,
    pub auto_download: bool,
    pub reader_mode: String,
    pub download_dir: String,
}

#[derive(Deserialize)]
pub struct SaveBody {
    pub download_workers: Option<i64>,
    pub index_interval_hours: Option<i64>,
    pub auto_download: Option<bool>,
    pub reader_mode: Option<String>,
    pub download_dir: Option<String>,
}

pub fn router() -> Router<AppState> {
    Router::new().route("/settings", get(get_settings).post(save_settings))
}

pub async fn read_settings(pool: &sqlx::SqlitePool, default_download_dir: &str) -> AppSettings {
    async fn val(pool: &sqlx::SqlitePool, key: &str) -> Option<String> {
        sqlx::query_scalar!("SELECT value FROM server_settings WHERE key = ?", key)
            .fetch_optional(pool)
            .await
            .map_err(|e| tracing::warn!("settings read error for '{}': {}", key, e))
            .ok()
            .flatten()
    }
    AppSettings {
        download_workers: val(pool, "download_workers").await
            .and_then(|v| v.parse().ok())
            .unwrap_or(2),
        index_interval_hours: val(pool, "index_interval_hours").await
            .and_then(|v| v.parse().ok())
            .unwrap_or(6),
        auto_download: val(pool, "auto_download").await
            .map(|v| v == "true")
            .unwrap_or(false),
        reader_mode: val(pool, "reader_mode").await
            .unwrap_or_else(|| "paged".to_string()),
        download_dir: val(pool, "download_dir").await
            .unwrap_or_else(|| default_download_dir.to_string()),
    }
}

async fn get_settings(State(state): State<AppState>) -> Json<AppSettings> {
    Json(read_settings(&state.db, &state.config.download_dir).await)
}

async fn save_settings(
    State(state): State<AppState>,
    Json(body): Json<SaveBody>,
) -> ApiResult<Response> {
    async fn upsert(pool: &sqlx::SqlitePool, key: &str, val: &str) -> sqlx::Result<()> {
        sqlx::query!(
            "INSERT INTO server_settings (key, value) VALUES (?, ?)
             ON CONFLICT(key) DO UPDATE SET value = excluded.value",
            key,
            val
        )
        .execute(pool)
        .await?;
        Ok(())
    }

    if let Some(w) = body.download_workers {
        upsert(&state.db, "download_workers", &w.to_string()).await?;
    }
    if let Some(h) = body.index_interval_hours {
        upsert(&state.db, "index_interval_hours", &h.to_string()).await?;
    }
    if let Some(a) = body.auto_download {
        upsert(&state.db, "auto_download", if a { "true" } else { "false" }).await?;
    }
    if let Some(ref m) = body.reader_mode {
        if !matches!(m.as_str(), "paged" | "scroll") {
            return Ok(StatusCode::UNPROCESSABLE_ENTITY.into_response());
        }
        upsert(&state.db, "reader_mode", m).await?;
    }
    if let Some(ref d) = body.download_dir {
        let d = d.trim();
        if !d.is_empty() {
            upsert(&state.db, "download_dir", d).await?;
        }
    }

    Ok(Json(read_settings(&state.db, &state.config.download_dir).await).into_response())
}
