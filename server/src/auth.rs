//! JWT issuance + verification (ADR 0033, S1 #123 / S2 #124). Tokens are
//! cross-compatible with the .NET server in both directions — Rust-issued
//! tokens must still validate under .NET's still-running `JwtBearer`
//! middleware for every route group not yet migrated, and vice versa. That
//! means matching the exact wire shape .NET's `JwtSecurityToken(claims: ...)`
//! constructor produces: claim keys are the literal long `ClaimTypes` URIs
//! (no short-name outbound mapping applies when claims are passed directly,
//! confirmed against a live .NET-issued token), `allow_explicit` is the
//! *string* `"true"`/`"false"` (a `Claim` value is always a string), and the
//! signing algorithm is HS256.

use std::time::{SystemTime, UNIX_EPOCH};

use axum::extract::FromRequestParts;
use axum::http::header::AUTHORIZATION;
use axum::http::request::Parts;
use jsonwebtoken::{decode, encode, Algorithm, DecodingKey, EncodingKey, Header, Validation};
use serde::{Deserialize, Serialize};

use crate::error::AppError;
use crate::state::AppState;

const TOKEN_LIFETIME_SECS: u64 = 30 * 24 * 3600; // 30 days, matches .NET

#[derive(Deserialize)]
pub struct Claims {
    #[serde(rename = "http://schemas.xmlsoap.org/ws/2005/05/identity/claims/nameidentifier")]
    pub user_id: String,
    #[serde(rename = "http://schemas.microsoft.com/ws/2008/06/identity/claims/role")]
    pub role: String,
}

impl Claims {
    pub fn require_admin(&self) -> Result<(), AppError> {
        if self.role == "admin" {
            Ok(())
        } else {
            Err(AppError::Forbidden)
        }
    }
}

#[derive(Serialize)]
struct OutboundClaims<'a> {
    #[serde(rename = "http://schemas.xmlsoap.org/ws/2005/05/identity/claims/nameidentifier")]
    sub: &'a str,
    username: &'a str,
    #[serde(rename = "http://schemas.microsoft.com/ws/2008/06/identity/claims/role")]
    role: &'a str,
    allow_explicit: &'static str,
    exp: u64,
}

pub fn create_token(
    user_id: &str,
    username: &str,
    role: &str,
    allow_explicit: bool,
    secret: &str,
) -> anyhow::Result<String> {
    let exp = SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs() + TOKEN_LIFETIME_SECS;
    let claims = OutboundClaims {
        sub: user_id,
        username,
        role,
        allow_explicit: if allow_explicit { "true" } else { "false" },
        exp,
    };
    Ok(encode(
        &Header::default(),
        &claims,
        &EncodingKey::from_secret(secret.as_bytes()),
    )?)
}

pub fn hash_password(password: &str) -> anyhow::Result<String> {
    Ok(bcrypt::hash(password, bcrypt::DEFAULT_COST)?)
}

pub fn verify_password(password: &str, hash: &str) -> bool {
    bcrypt::verify(password, hash).unwrap_or(false)
}

impl FromRequestParts<AppState> for Claims {
    type Rejection = AppError;

    async fn from_request_parts(
        parts: &mut Parts,
        state: &AppState,
    ) -> Result<Self, Self::Rejection> {
        let secret = state
            .config
            .jwt_secret
            .as_deref()
            .ok_or(AppError::Unauthorized)?;

        let token = parts
            .headers
            .get(AUTHORIZATION)
            .and_then(|v| v.to_str().ok())
            .and_then(|v| v.strip_prefix("Bearer "))
            .ok_or(AppError::Unauthorized)?;

        decode::<Claims>(
            token,
            &DecodingKey::from_secret(secret.as_bytes()),
            &Validation::new(Algorithm::HS256),
        )
        .map(|data| data.claims)
        .map_err(|_| AppError::Unauthorized)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn create_token_roundtrip_admin_claims() {
        let token = create_token("user-1", "alice", "admin", true, "s3cr3t").unwrap();
        let data = decode::<Claims>(
            &token,
            &DecodingKey::from_secret(b"s3cr3t"),
            &Validation::new(Algorithm::HS256),
        )
        .unwrap();
        assert_eq!(data.claims.user_id, "user-1");
        assert_eq!(data.claims.role, "admin");
    }

    #[test]
    fn create_token_wrong_secret_rejected() {
        let token = create_token("user-1", "alice", "admin", true, "s3cr3t").unwrap();
        let result = decode::<Claims>(
            &token,
            &DecodingKey::from_secret(b"wrong"),
            &Validation::new(Algorithm::HS256),
        );
        assert!(result.is_err());
    }

    #[test]
    fn require_admin_rejects_member() {
        let claims = Claims {
            user_id: "user-2".into(),
            role: "member".into(),
        };
        assert!(matches!(claims.require_admin(), Err(AppError::Forbidden)));
    }

    #[test]
    fn password_hash_roundtrip() {
        let hash = hash_password("secret123").unwrap();
        assert!(verify_password("secret123", &hash));
        assert!(!verify_password("wrong", &hash));
    }

    /// Cross-compat: verify a hash BCrypt.Net-Next actually produced (`$2a$`
    /// variant, cost 11) against a live .NET-created user row.
    #[test]
    fn verifies_dotnet_bcrypt_hash() {
        let dotnet_hash = "$2a$11$DTQZsXk/3YwBcThxy/t3k.jW3fY/e5u8QyJ/rTO4jslPfdf091HKi";
        assert!(verify_password("probepass", dotnet_hash));
        assert!(!verify_password("wrongpass", dotnet_hash));
    }
}
