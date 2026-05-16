use axum::{
    extract::State,
    http::StatusCode,
    response::{IntoResponse, Json, Response},
    routing::{get, post},
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
}

#[derive(Serialize)]
pub struct MeResponse {
    pub id: String,
    pub username: String,
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

    sqlx::query!(
        "INSERT INTO users (id, username, password_hash, created_at) VALUES (?, ?, ?, ?)",
        id, username, password_hash, now
    )
    .execute(&state.db)
    .await?;

    let token = auth::create_token(&id, &username, &state.jwt_secret)?;

    Ok(Json(AuthResponse { token, username, user_id: id }).into_response())
}

async fn login(
    State(state): State<AppState>,
    Json(body): Json<LoginBody>,
) -> ApiResult<Response> {
    let row = sqlx::query!(
        "SELECT id, username, password_hash FROM users WHERE username = ?",
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

    let token = auth::create_token(&row.id, &row.username, &state.jwt_secret)?;

    Ok(Json(AuthResponse { token, username: row.username, user_id: row.id }).into_response())
}

async fn me(
    State(state): State<AppState>,
    axum::Extension(claims): axum::Extension<auth::Claims>,
) -> ApiResult<Response> {
    let row = sqlx::query!("SELECT id, username FROM users WHERE id = ?", claims.sub)
        .fetch_optional(&state.db)
        .await?;

    let Some(row) = row else {
        return Ok(StatusCode::NOT_FOUND.into_response());
    };
    Ok(Json(MeResponse { id: row.id, username: row.username }).into_response())
}
