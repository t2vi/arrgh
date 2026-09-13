//! `/api/users/*` — admin-only user management, port of `Api/Auth.cs`'s
//! user-CRUD half (ADR 0033, S2 #124).

use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::routing::{get, patch};
use axum::{Json, Router};
use serde::{Deserialize, Serialize};

use crate::auth::{self, Claims};
use crate::error::{AppError, AppResult};
use crate::state::AppState;
use crate::users;

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/", get(list).post(create))
        .route("/{id}", patch(patch_user).delete(delete_user))
}

#[derive(Serialize)]
struct UserListItem {
    id: String,
    username: String,
    role: String,
    allow_explicit: bool,
    created_at: String,
}

impl From<users::UserRow> for UserListItem {
    fn from(u: users::UserRow) -> Self {
        Self {
            id: u.id,
            username: u.username,
            role: u.role,
            allow_explicit: u.allow_explicit,
            // EF's own JSON view re-formats to ISO-ish "o"; not asserted by
            // any test, so the raw "yyyy-MM-dd HH:mm:ss.ffffff" storage
            // format (T-separated) is close enough rather than replicating
            // .NET's exact round-trip format here.
            created_at: u.created_at.replacen(' ', "T", 1),
        }
    }
}

async fn list(claims: Claims, State(state): State<AppState>) -> AppResult<Json<Vec<UserListItem>>> {
    claims.require_admin()?;
    let rows = users::list_ordered_by_created_at(&state.db).await?;
    Ok(Json(rows.into_iter().map(UserListItem::from).collect()))
}

#[derive(Deserialize)]
struct CreateUserBody {
    username: String,
    password: String,
}

async fn create(
    claims: Claims,
    State(state): State<AppState>,
    Json(body): Json<CreateUserBody>,
) -> AppResult<StatusCode> {
    claims.require_admin()?;
    if body.username.trim().is_empty() || body.password.len() < 6 {
        return Err(AppError::UnprocessableEntity(
            "invalid username or password".into(),
        ));
    }
    if users::exists_by_username(&state.db, &body.username).await? {
        return Err(AppError::Conflict("username already exists".into()));
    }

    let id = uuid::Uuid::new_v4().to_string();
    let hash = auth::hash_password(&body.password)?;
    users::insert(&state.db, &id, body.username.trim(), &hash, "member", false).await?;
    Ok(StatusCode::CREATED)
}

#[derive(Deserialize)]
struct PatchUserBody {
    role: Option<String>,
    allow_explicit: Option<bool>,
    password: Option<String>,
}

async fn patch_user(
    claims: Claims,
    State(state): State<AppState>,
    Path(id): Path<String>,
    Json(body): Json<PatchUserBody>,
) -> AppResult<StatusCode> {
    claims.require_admin()?;
    users::find_by_id(&state.db, &id)
        .await?
        .ok_or(AppError::NotFound)?;

    if let Some(role) = &body.role {
        if role != "admin" && role != "member" {
            return Err(AppError::UnprocessableEntity("invalid role".into()));
        }
        users::update_role(&state.db, &id, role).await?;
    }
    if let Some(allow_explicit) = body.allow_explicit {
        users::update_allow_explicit(&state.db, &id, allow_explicit).await?;
    }
    if let Some(password) = &body.password {
        if password.len() < 6 {
            return Err(AppError::UnprocessableEntity("password too short".into()));
        }
        let hash = auth::hash_password(password)?;
        users::update_password(&state.db, &id, &hash).await?;
    }

    Ok(StatusCode::NO_CONTENT)
}

async fn delete_user(
    claims: Claims,
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> AppResult<StatusCode> {
    claims.require_admin()?;
    if claims.user_id == id {
        return Err(AppError::Forbidden);
    }
    if !users::delete(&state.db, &id).await? {
        return Err(AppError::NotFound);
    }
    Ok(StatusCode::NO_CONTENT)
}
