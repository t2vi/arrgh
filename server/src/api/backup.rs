//! `/api/backups/*` — admin-only DB backup + restore (spec 035, issue #251). See
//! `specs/035-db-backup-restore/contracts/db-backup-restore.md`.

use std::path::{Path, PathBuf};

use axum::extract::{Multipart, Path as AxumPath, State};
use axum::http::StatusCode;
use axum::routing::{get, post};
use axum::{Json, Router};
use tokio::io::AsyncWriteExt;

use crate::auth::Claims;
use crate::backup::{self, BackupInfo, RestoreRejection};
use crate::error::{AppError, AppResult};
use crate::settings;
use crate::state::AppState;

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/", get(list).post(create))
        .route("/{filename}", axum::routing::delete(delete_one))
        .route("/{filename}/restore", post(restore))
        .route("/restore-upload", post(restore_upload))
}

async fn backup_dir(state: &AppState) -> sqlx::Result<Option<PathBuf>> {
    let raw = settings::get(&state.db, settings::BACKUP_DIR).await?;
    Ok(raw
        .filter(|v| !v.trim().is_empty())
        .map(|v| PathBuf::from(v.trim())))
}

async fn list(claims: Claims, State(state): State<AppState>) -> AppResult<Json<Vec<BackupInfo>>> {
    claims.require_admin()?;
    let Some(dir) = backup_dir(&state).await.map_err(anyhow::Error::from)? else {
        return Ok(Json(Vec::new()));
    };
    let backups = backup::list_backups(&dir)
        .await
        .map_err(anyhow::Error::from)?;
    Ok(Json(backups))
}

async fn create(
    claims: Claims,
    State(state): State<AppState>,
) -> AppResult<(StatusCode, Json<BackupInfo>)> {
    claims.require_admin()?;
    let dir = backup_dir(&state)
        .await
        .map_err(anyhow::Error::from)?
        .ok_or_else(|| {
            AppError::UnprocessableEntity(
                "no backup destination configured — set one in Settings first".into(),
            )
        })?;
    let info = backup::create_backup(&state.db, &dir).await?;
    Ok((StatusCode::CREATED, Json(info)))
}

async fn delete_one(
    claims: Claims,
    State(state): State<AppState>,
    AxumPath(filename): AxumPath<String>,
) -> AppResult<StatusCode> {
    claims.require_admin()?;
    let dir = backup_dir(&state)
        .await
        .map_err(anyhow::Error::from)?
        .ok_or(AppError::NotFound)?;
    if backup::delete_backup(&dir, &filename).await? {
        Ok(StatusCode::NO_CONTENT)
    } else {
        Err(AppError::NotFound)
    }
}

/// Where a restore candidate is staged before validation — must be in the live DB file's own
/// directory so the final swap (`std::fs::rename`) is atomic (same filesystem).
fn temp_candidate_path(state: &AppState) -> PathBuf {
    let db_path = Path::new(&state.config.database_path);
    let dir = db_path.parent().filter(|p| !p.as_os_str().is_empty());
    let name = format!(".arrgh-restore-{}.tmp", uuid::Uuid::new_v4());
    match dir {
        Some(d) => d.join(name),
        None => PathBuf::from(name),
    }
}

/// Validates `candidate`, swaps it in on success, restarts the process (unless
/// `Config::restart_on_restore` is `false` — the test seam). Deletes `candidate` and leaves the
/// live database untouched on rejection.
async fn finish_restore(state: &AppState, candidate: PathBuf) -> AppResult<StatusCode> {
    match backup::validate_candidate(&candidate).await {
        Ok(()) => {
            backup::swap_in(&candidate, &state.config.database_path)
                .map_err(anyhow::Error::from)?;
            if state.config.restart_on_restore {
                tokio::spawn(async {
                    tokio::time::sleep(std::time::Duration::from_millis(200)).await;
                    std::process::exit(0);
                });
            }
            Ok(StatusCode::ACCEPTED)
        }
        Err(rejection) => {
            let _ = tokio::fs::remove_file(&candidate).await;
            Err(match rejection {
                RestoreRejection::Invalid => AppError::UnprocessableEntity("invalid_backup".into()),
                RestoreRejection::TooNew => AppError::UnprocessableEntity("backup_too_new".into()),
            })
        }
    }
}

async fn restore(
    claims: Claims,
    State(state): State<AppState>,
    AxumPath(filename): AxumPath<String>,
) -> AppResult<StatusCode> {
    claims.require_admin()?;
    if !backup::is_backup_filename(&filename) {
        return Err(AppError::NotFound);
    }
    let dir = backup_dir(&state)
        .await
        .map_err(anyhow::Error::from)?
        .ok_or(AppError::NotFound)?;
    let src = dir.join(&filename);
    if !tokio::fs::try_exists(&src).await.unwrap_or(false) {
        return Err(AppError::NotFound);
    }
    let candidate = temp_candidate_path(&state);
    tokio::fs::copy(&src, &candidate)
        .await
        .map_err(anyhow::Error::from)?;
    finish_restore(&state, candidate).await
}

async fn restore_upload(
    claims: Claims,
    State(state): State<AppState>,
    mut multipart: Multipart,
) -> AppResult<StatusCode> {
    claims.require_admin()?;
    let candidate = temp_candidate_path(&state);
    let mut wrote_any = false;
    while let Some(mut field) = multipart
        .next_field()
        .await
        .map_err(|e| AppError::UnprocessableEntity(e.to_string()))?
    {
        if field.file_name().is_none() {
            continue;
        }
        let mut file = tokio::fs::File::create(&candidate)
            .await
            .map_err(anyhow::Error::from)?;
        while let Some(chunk) = field
            .chunk()
            .await
            .map_err(|e| AppError::UnprocessableEntity(e.to_string()))?
        {
            file.write_all(&chunk).await.map_err(anyhow::Error::from)?;
        }
        wrote_any = true;
        break;
    }
    if !wrote_any {
        return Err(AppError::UnprocessableEntity(
            "no file field in upload".into(),
        ));
    }
    finish_restore(&state, candidate).await
}
