use anyhow::Result;
use chrono::{Duration, Utc};
use jsonwebtoken::{decode, encode, DecodingKey, EncodingKey, Header, Validation};
use serde::{Deserialize, Serialize};

const TOKEN_DAYS: i64 = 30;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Claims {
    pub sub: String,
    pub username: String,
    pub role: String,
    pub allow_explicit: bool,
    pub exp: usize,
}

pub fn create_token(user_id: &str, username: &str, role: &str, allow_explicit: bool, secret: &str) -> Result<String> {
    let exp = (Utc::now() + Duration::days(TOKEN_DAYS)).timestamp() as usize;
    let claims = Claims {
        sub: user_id.to_string(),
        username: username.to_string(),
        role: role.to_string(),
        allow_explicit,
        exp,
    };
    Ok(encode(
        &Header::default(),
        &claims,
        &EncodingKey::from_secret(secret.as_bytes()),
    )?)
}

pub fn validate_token(token: &str, secret: &str) -> Result<Claims> {
    let data = decode::<Claims>(
        token,
        &DecodingKey::from_secret(secret.as_bytes()),
        &Validation::default(),
    )?;
    Ok(data.claims)
}

#[cfg(test)]
mod tests {
    use super::*;

    const SECRET: &str = "test-secret";

    #[test]
    fn roundtrip_token() {
        let token = create_token("user-1", "alice", "admin", true, SECRET).unwrap();
        let claims = validate_token(&token, SECRET).unwrap();
        assert_eq!(claims.sub, "user-1");
        assert_eq!(claims.username, "alice");
        assert_eq!(claims.role, "admin");
        assert!(claims.allow_explicit);
    }

    #[test]
    fn wrong_secret_rejected() {
        let token = create_token("user-1", "alice", "member", false, SECRET).unwrap();
        assert!(validate_token(&token, "wrong-secret").is_err());
    }

    #[test]
    fn member_role_preserved() {
        let token = create_token("u", "bob", "member", false, SECRET).unwrap();
        let claims = validate_token(&token, SECRET).unwrap();
        assert_eq!(claims.role, "member");
        assert!(!claims.allow_explicit);
    }
}
