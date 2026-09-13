//! JWT verification for protected routes. Full auth (login/register/users)
//! lands in S2 (#124) — this only decodes the token the .NET server already
//! issues, so `/api/logs` (S1, #123) can gate on it during the strangler-fig.

use axum::extract::FromRequestParts;
use axum::http::header::AUTHORIZATION;
use axum::http::request::Parts;
use jsonwebtoken::{decode, Algorithm, DecodingKey, Validation};
use serde::Deserialize;

use crate::error::AppError;
use crate::state::AppState;

/// .NET signs claims under the long `ClaimTypes` URIs (verified against a
/// live token) — `role` is the only one any route needs so far.
#[derive(Deserialize)]
pub struct Claims {
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
    use jsonwebtoken::{encode, EncodingKey, Header};
    use serde::Serialize;

    use super::*;

    #[derive(Serialize)]
    struct TestClaims<'a> {
        #[serde(rename = "http://schemas.microsoft.com/ws/2008/06/identity/claims/role")]
        role: &'a str,
        exp: usize,
    }

    fn mint(role: &str, secret: &str) -> String {
        let claims = TestClaims {
            role,
            exp: 9_999_999_999,
        };
        encode(
            &Header::default(),
            &claims,
            &EncodingKey::from_secret(secret.as_bytes()),
        )
        .unwrap()
    }

    #[test]
    fn decodes_role_from_dotnet_shaped_claim() {
        let token = mint("admin", "s3cr3t");
        let data = decode::<Claims>(
            &token,
            &DecodingKey::from_secret(b"s3cr3t"),
            &Validation::new(Algorithm::HS256),
        )
        .unwrap();
        assert_eq!(data.claims.role, "admin");
    }

    #[test]
    fn require_admin_rejects_member() {
        let claims = Claims {
            role: "member".into(),
        };
        assert!(matches!(claims.require_admin(), Err(AppError::Forbidden)));
    }

    #[test]
    fn wrong_secret_rejected() {
        let token = mint("admin", "s3cr3t");
        let result = decode::<Claims>(
            &token,
            &DecodingKey::from_secret(b"wrong"),
            &Validation::new(Algorithm::HS256),
        );
        assert!(result.is_err());
    }
}
