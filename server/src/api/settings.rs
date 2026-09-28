//! `/api/settings` — port of `Api/Settings.cs` (ADR 0033, S3 #125). No auth
//! required, matching the .NET route (unauthenticated on purpose).

use axum::extract::State;
use axum::routing::get;
use axum::{Json, Router};
use serde::{Deserialize, Serialize};

use crate::error::{AppError, AppResult};
use crate::settings;
use crate::state::AppState;

pub fn routes() -> Router<AppState> {
    Router::new().route("/", get(get_settings).post(save_settings))
}

#[derive(Serialize)]
struct AppSettingsDto {
    download_workers: i64,
    index_interval_hours: i64,
    auto_download: bool,
    reader_mode: String,
    download_dir: String,
    trending_per_source: i64,
    check_for_updates: bool,
    backup_dir: String,
    backup_interval_hours: i64,
}

async fn read_settings(state: &AppState) -> AppResult<AppSettingsDto> {
    let kv = settings::get_all(&state.db).await?;
    let get = |k: &str| kv.get(k).map(String::as_str);

    Ok(AppSettingsDto {
        download_workers: settings::parse_long(
            get(settings::DOWNLOAD_WORKERS),
            settings::DEFAULT_DOWNLOAD_WORKERS,
        ),
        index_interval_hours: settings::parse_long(
            get(settings::INDEX_INTERVAL_HOURS),
            settings::DEFAULT_INDEX_INTERVAL_HOURS,
        ),
        auto_download: settings::parse_bool(
            get(settings::AUTO_DOWNLOAD),
            settings::DEFAULT_AUTO_DOWNLOAD,
        ),
        reader_mode: get("reader_mode").unwrap_or("scroll").to_string(),
        download_dir: get(settings::DOWNLOAD_DIR)
            .map(str::to_string)
            .unwrap_or_else(|| state.config.download_dir.clone()),
        trending_per_source: settings::parse_long(
            get(settings::TRENDING_PER_SOURCE),
            settings::DEFAULT_TRENDING_PER_SOURCE,
        ),
        check_for_updates: settings::parse_bool(get("check_for_updates"), false),
        backup_dir: get(settings::BACKUP_DIR).unwrap_or("").to_string(),
        backup_interval_hours: settings::parse_long(
            get(settings::BACKUP_INTERVAL_HOURS),
            settings::DEFAULT_BACKUP_INTERVAL_HOURS,
        ),
    })
}

async fn get_settings(State(state): State<AppState>) -> AppResult<Json<AppSettingsDto>> {
    Ok(Json(read_settings(&state).await?))
}

#[derive(Deserialize)]
struct SaveSettingsBody {
    download_workers: Option<i64>,
    index_interval_hours: Option<i64>,
    auto_download: Option<bool>,
    reader_mode: Option<String>,
    download_dir: Option<String>,
    trending_per_source: Option<i64>,
    check_for_updates: Option<bool>,
    backup_dir: Option<String>,
    backup_interval_hours: Option<i64>,
}

async fn save_settings(
    State(state): State<AppState>,
    Json(body): Json<SaveSettingsBody>,
) -> AppResult<Json<AppSettingsDto>> {
    if let Some(mode) = &body.reader_mode {
        if !settings::valid_reader_mode(mode) {
            return Err(AppError::UnprocessableEntity("invalid reader_mode".into()));
        }
    }

    if let Some(v) = body.download_workers {
        settings::set(&state.db, settings::DOWNLOAD_WORKERS, &v.to_string()).await?;
    }
    if let Some(v) = body.index_interval_hours {
        settings::set(&state.db, settings::INDEX_INTERVAL_HOURS, &v.to_string()).await?;
    }
    if let Some(v) = body.auto_download {
        settings::set(
            &state.db,
            settings::AUTO_DOWNLOAD,
            if v { "true" } else { "false" },
        )
        .await?;
    }
    if let Some(v) = &body.reader_mode {
        settings::set(&state.db, "reader_mode", v).await?;
    }
    if let Some(v) = &body.download_dir {
        let trimmed = v.trim();
        if !trimmed.is_empty() {
            settings::set(&state.db, settings::DOWNLOAD_DIR, trimmed).await?;
        }
    }
    if let Some(v) = body.trending_per_source {
        settings::set(
            &state.db,
            settings::TRENDING_PER_SOURCE,
            &settings::clamp_trending(v).to_string(),
        )
        .await?;
    }
    if let Some(v) = body.check_for_updates {
        settings::set(
            &state.db,
            "check_for_updates",
            if v { "true" } else { "false" },
        )
        .await?;
    }
    if let Some(v) = &body.backup_dir {
        // Unlike download_dir, an empty value is meaningful here — it's how an admin turns
        // scheduled/manual backups back off (settings::BACKUP_DIR has no safe default to fall
        // back to).
        settings::set(&state.db, settings::BACKUP_DIR, v.trim()).await?;
    }
    if let Some(v) = body.backup_interval_hours {
        settings::set(&state.db, settings::BACKUP_INTERVAL_HOURS, &v.to_string()).await?;
    }

    Ok(Json(read_settings(&state).await?))
}
