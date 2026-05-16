use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::{IntoResponse, Json, Response},
    routing::{get, patch, post},
    Router,
};
use bcrypt::{hash, verify, DEFAULT_COST};
use chrono::Utc;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::{auth, AppState};
use super::ApiResult;

#[derive(Serialize)]
pub struct AuthStatus {
    pub needs_setup: bool,
}

#[derive(Deserialize)]
pub struct RegisterBody {
    pub username: String,
    pub password: String,
}

#[derive(Deserialize)]
pub struct LoginBody {
    pub username: String,
    pub password: String,
}

#[derive(Serialize)]
pub struct AuthResponse {
    pub token: String,
    pub username: String,
    pub user_id: String,
    pub role: String,
    pub allow_explicit: bool,
}

#[derive(Serialize)]
pub struct MeResponse {
    pub id: String,
    pub username: String,
    pub role: String,
    pub allow_explicit: bool,
}

#[derive(Serialize)]
pub struct UserListItem {
    pub id: String,
    pub username: String,
    pub role: String,
    pub allow_explicit: bool,
    pub created_at: String,
}

#[derive(Deserialize)]
pub struct CreateUserBody {
    pub username: String,
    pub password: String,
}

#[derive(Deserialize)]
pub struct PatchUserBody {
    pub role: Option<String>,
    pub allow_explicit: Option<bool>,
    pub password: Option<String>,
}

/// Public routes — no auth middleware
pub fn public_router() -> Router<AppState> {
    Router::new()
        .route("/auth/status", get(status))
        .route("/auth/register", post(register))
        .route("/auth/login", post(login))
}

/// Protected routes — behind auth middleware in api/mod.rs
pub fn protected_router() -> Router<AppState> {
    Router::new()
        .route("/auth/me", get(me))
        .route("/users", get(list_users).post(create_user))
        .route("/users/{id}", patch(patch_user).delete(delete_user))
}

async fn status(
    State(state): State<AppState>,
) -> ApiResult<Json<AuthStatus>> {
    let count = sqlx::query_scalar!("SELECT COUNT(*) FROM users")
        .fetch_one(&state.db)
        .await?;

    Ok(Json(AuthStatus { needs_setup: count == 0 }))
}

async fn register(
    State(state): State<AppState>,
    Json(body): Json<RegisterBody>,
) -> ApiResult<Response> {
    let count = sqlx::query_scalar!("SELECT COUNT(*) FROM users")
        .fetch_one(&state.db)
        .await?;

    if count > 0 {
        return Ok(StatusCode::FORBIDDEN.into_response());
    }

    if body.username.trim().is_empty() || body.password.len() < 6 {
        return Ok(StatusCode::UNPROCESSABLE_ENTITY.into_response());
    }

    let password_hash = hash(&body.password, DEFAULT_COST)?;

    let id = Uuid::new_v4().to_string();
    let now = Utc::now();
    let username = body.username.trim().to_string();
    let role = "admin";

    sqlx::query!(
        "INSERT INTO users (id, username, password_hash, role, allow_explicit, created_at) VALUES (?, ?, ?, ?, 1, ?)",
        id, username, password_hash, role, now
    )
    .execute(&state.db)
    .await?;

    let token = auth::create_token(&id, &username, role, true, &state.jwt_secret)?;

    Ok(Json(AuthResponse { token, username, user_id: id, role: role.to_string(), allow_explicit: true }).into_response())
}

async fn login(
    State(state): State<AppState>,
    Json(body): Json<LoginBody>,
) -> ApiResult<Response> {
    let row = sqlx::query!(
        "SELECT id, username, password_hash, role, allow_explicit FROM users WHERE username = ?",
        body.username
    )
    .fetch_optional(&state.db)
    .await?;

    let Some(row) = row else {
        return Ok(StatusCode::UNAUTHORIZED.into_response());
    };

    let valid = verify(&body.password, &row.password_hash)?;
    if !valid {
        return Ok(StatusCode::UNAUTHORIZED.into_response());
    }

    let allow_explicit = row.allow_explicit != 0;
    let token = auth::create_token(&row.id, &row.username, &row.role, allow_explicit, &state.jwt_secret)?;

    Ok(Json(AuthResponse {
        token,
        username: row.username,
        user_id: row.id,
        role: row.role,
        allow_explicit,
    }).into_response())
}

async fn me(
    State(state): State<AppState>,
    axum::Extension(claims): axum::Extension<auth::Claims>,
) -> ApiResult<Response> {
    let row = sqlx::query!(
        "SELECT id, username, role, allow_explicit FROM users WHERE id = ?",
        claims.sub
    )
    .fetch_optional(&state.db)
    .await?;

    let Some(row) = row else {
        return Ok(StatusCode::NOT_FOUND.into_response());
    };
    Ok(Json(MeResponse {
        id: row.id,
        username: row.username,
        role: row.role,
        allow_explicit: row.allow_explicit != 0,
    }).into_response())
}

async fn list_users(
    State(state): State<AppState>,
    axum::Extension(claims): axum::Extension<auth::Claims>,
) -> ApiResult<Response> {
    if claims.role != "admin" {
        return Ok(StatusCode::FORBIDDEN.into_response());
    }

    let rows = sqlx::query!(
        "SELECT id, username, role, allow_explicit, created_at FROM users ORDER BY created_at"
    )
    .fetch_all(&state.db)
    .await?;

    let users: Vec<UserListItem> = rows.into_iter().map(|r| UserListItem {
        id: r.id,
        username: r.username,
        role: r.role,
        allow_explicit: r.allow_explicit != 0,
        created_at: r.created_at.to_string(),
    }).collect();

    Ok(Json(users).into_response())
}

async fn create_user(
    State(state): State<AppState>,
    axum::Extension(claims): axum::Extension<auth::Claims>,
    Json(body): Json<CreateUserBody>,
) -> ApiResult<Response> {
    if claims.role != "admin" {
        return Ok(StatusCode::FORBIDDEN.into_response());
    }

    if body.username.trim().is_empty() || body.password.len() < 6 {
        return Ok(StatusCode::UNPROCESSABLE_ENTITY.into_response());
    }

    let exists = sqlx::query_scalar!(
        "SELECT COUNT(*) FROM users WHERE username = ?",
        body.username
    )
    .fetch_one(&state.db)
    .await?;

    if exists > 0 {
        return Ok(StatusCode::CONFLICT.into_response());
    }

    let password_hash = hash(&body.password, DEFAULT_COST)?;
    let id = Uuid::new_v4().to_string();
    let now = Utc::now();
    let username = body.username.trim().to_string();

    sqlx::query!(
        "INSERT INTO users (id, username, password_hash, role, allow_explicit, created_at) VALUES (?, ?, ?, 'member', 0, ?)",
        id, username, password_hash, now
    )
    .execute(&state.db)
    .await?;

    Ok(StatusCode::CREATED.into_response())
}

async fn patch_user(
    State(state): State<AppState>,
    axum::Extension(claims): axum::Extension<auth::Claims>,
    Path(id): Path<String>,
    Json(body): Json<PatchUserBody>,
) -> ApiResult<Response> {
    if claims.role != "admin" {
        return Ok(StatusCode::FORBIDDEN.into_response());
    }

    let exists = sqlx::query_scalar!("SELECT COUNT(*) FROM users WHERE id = ?", id)
        .fetch_one(&state.db)
        .await?;
    if exists == 0 {
        return Ok(StatusCode::NOT_FOUND.into_response());
    }

    if let Some(role) = &body.role {
        if role != "admin" && role != "member" {
            return Ok(StatusCode::UNPROCESSABLE_ENTITY.into_response());
        }
        sqlx::query!("UPDATE users SET role = ? WHERE id = ?", role, id)
            .execute(&state.db)
            .await?;
    }

    if let Some(allow) = body.allow_explicit {
        let v = if allow { 1 } else { 0 };
        sqlx::query!("UPDATE users SET allow_explicit = ? WHERE id = ?", v, id)
            .execute(&state.db)
            .await?;
    }

    if let Some(pw) = &body.password {
        if pw.len() < 6 {
            return Ok(StatusCode::UNPROCESSABLE_ENTITY.into_response());
        }
        let h = hash(pw, DEFAULT_COST)?;
        sqlx::query!("UPDATE users SET password_hash = ? WHERE id = ?", h, id)
            .execute(&state.db)
            .await?;
    }

    Ok(StatusCode::NO_CONTENT.into_response())
}

async fn delete_user(
    State(state): State<AppState>,
    axum::Extension(claims): axum::Extension<auth::Claims>,
    Path(id): Path<String>,
) -> ApiResult<Response> {
    if claims.role != "admin" {
        return Ok(StatusCode::FORBIDDEN.into_response());
    }

    if claims.sub == id {
        return Ok(StatusCode::FORBIDDEN.into_response());
    }

    let result = sqlx::query!("DELETE FROM users WHERE id = ?", id)
        .execute(&state.db)
        .await?;

    if result.rows_affected() == 0 {
        return Ok(StatusCode::NOT_FOUND.into_response());
    }

    Ok(StatusCode::NO_CONTENT.into_response())
}
