//! `/api/auth/*` — port of `Api/Auth.cs`'s auth half (ADR 0033, S2 #124).
//! Status/register/login are public; `/me` requires a valid token.

use axum::extract::State;
use axum::routing::{get, post};
use axum::{Json, Router};
use serde::{Deserialize, Serialize};

use crate::auth::{self, Claims};
use crate::error::{AppError, AppResult};
use crate::state::AppState;
use crate::users;

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/status", get(status))
        .route("/register", post(register))
        .route("/login", post(login))
        .route("/me", get(me).patch(patch_me))
}

#[derive(Serialize)]
struct StatusResponse {
    needs_setup: bool,
}

async fn status(State(state): State<AppState>) -> AppResult<Json<StatusResponse>> {
    let count = users::count(&state.db).await?;
    Ok(Json(StatusResponse {
        needs_setup: count == 0,
    }))
}

#[derive(Deserialize)]
struct RegisterBody {
    username: String,
    password: String,
}

#[derive(Serialize)]
struct AuthResponse {
    token: String,
    username: String,
    user_id: String,
    role: String,
    allow_explicit: bool,
}

async fn register(
    State(state): State<AppState>,
    Json(body): Json<RegisterBody>,
) -> AppResult<Json<AuthResponse>> {
    let count = users::count(&state.db).await?;
    if count > 0 {
        return Err(AppError::Forbidden);
    }
    if body.username.trim().is_empty() || body.password.len() < 6 {
        return Err(AppError::UnprocessableEntity(
            "invalid username or password".into(),
        ));
    }

    let id = uuid::Uuid::new_v4().to_string();
    let username = body.username.trim();
    let hash = auth::hash_password(&body.password)?;
    users::insert(&state.db, &id, username, &hash, "admin", true).await?;

    let secret = jwt_secret(&state)?;
    let token = auth::create_token(&id, username, "admin", true, secret)?;
    Ok(Json(AuthResponse {
        token,
        username: username.to_string(),
        user_id: id,
        role: "admin".into(),
        allow_explicit: true,
    }))
}

#[derive(Deserialize)]
struct LoginBody {
    username: String,
    password: String,
}

async fn login(
    State(state): State<AppState>,
    Json(body): Json<LoginBody>,
) -> AppResult<Json<AuthResponse>> {
    let user = users::find_by_username(&state.db, &body.username)
        .await?
        .ok_or(AppError::Unauthorized)?;

    if !auth::verify_password(&body.password, &user.password_hash) {
        return Err(AppError::Unauthorized);
    }

    let secret = jwt_secret(&state)?;
    let token = auth::create_token(
        &user.id,
        &user.username,
        &user.role,
        user.allow_explicit,
        secret,
    )?;
    Ok(Json(AuthResponse {
        token,
        username: user.username,
        user_id: user.id,
        role: user.role,
        allow_explicit: user.allow_explicit,
    }))
}

#[derive(Serialize)]
struct MeResponse {
    id: String,
    username: String,
    role: String,
    allow_explicit: bool,
}

async fn me(claims: Claims, State(state): State<AppState>) -> AppResult<Json<MeResponse>> {
    let user = users::find_by_id(&state.db, &claims.user_id)
        .await?
        .ok_or(AppError::NotFound)?;
    Ok(Json(MeResponse {
        id: user.id,
        username: user.username,
        role: user.role,
        allow_explicit: user.allow_explicit,
    }))
}

#[derive(Deserialize)]
struct PatchMeBody {
    password: Option<String>,
    allow_explicit: Option<bool>,
}

async fn patch_me(
    claims: Claims,
    State(state): State<AppState>,
    Json(body): Json<PatchMeBody>,
) -> AppResult<Json<MeResponse>> {
    let user = users::find_by_id(&state.db, &claims.user_id)
        .await?
        .ok_or(AppError::NotFound)?;

    if let Some(password) = &body.password {
        if password.len() < 6 {
            return Err(AppError::UnprocessableEntity("password too short".into()));
        }
        let hash = auth::hash_password(password)?;
        users::update_password(&state.db, &user.id, &hash).await?;
    }
    if let Some(allow_explicit) = body.allow_explicit {
        users::update_allow_explicit(&state.db, &user.id, allow_explicit).await?;
    }

    let updated = users::find_by_id(&state.db, &user.id)
        .await?
        .ok_or(AppError::NotFound)?;
    Ok(Json(MeResponse {
        id: updated.id,
        username: updated.username,
        role: updated.role,
        allow_explicit: updated.allow_explicit,
    }))
}

fn jwt_secret(state: &AppState) -> Result<&str, AppError> {
    state
        .config
        .jwt_secret
        .as_deref()
        .ok_or_else(|| AppError::Internal(anyhow::anyhow!("JwtSecret not configured")))
}
