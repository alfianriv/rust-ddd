//! Auth adapters: argon2id password hashing, HS256 JWT access tokens.
use app_core::shared::ports::{PasswordHasher, TokenService};
use app_core::shared::{DomainError, Result};
use app_core::user::UserId;
use argon2::Argon2;
use argon2::password_hash::{PasswordHasher as _, PasswordVerifier as _, phc::PasswordHash};
use async_trait::async_trait;
use jsonwebtoken::{Algorithm, DecodingKey, EncodingKey, Header, Validation};
use serde::{Deserialize, Serialize};

fn infra(e: impl std::fmt::Display) -> DomainError {
    DomainError::Infra(e.to_string())
}

/// Argon2id with the crate defaults (OWASP-recommended params).
/// Hashing runs on tokio's blocking pool so it never stalls request handling.
pub struct Argon2Hasher;

#[async_trait]
impl PasswordHasher for Argon2Hasher {
    async fn hash(&self, password: &str) -> Result<String> {
        let password = password.to_owned();
        tokio::task::spawn_blocking(move || {
            Argon2::default()
                .hash_password(password.as_bytes())
                .map(|h| h.to_string())
        })
        .await
        .map_err(infra)?
        .map_err(infra)
    }

    async fn verify(&self, password: &str, hash: &str) -> Result<bool> {
        let (password, hash) = (password.to_owned(), hash.to_owned());
        tokio::task::spawn_blocking(move || {
            let parsed = PasswordHash::new(&hash).map_err(infra)?;
            Ok(Argon2::default()
                .verify_password(password.as_bytes(), &parsed)
                .is_ok())
        })
        .await
        .map_err(infra)?
    }
}

#[derive(Serialize, Deserialize)]
struct Claims {
    sub: String,
    iat: i64,
    exp: i64,
}

pub struct JwtTokens {
    encoding: EncodingKey,
    decoding: DecodingKey,
    ttl_secs: i64,
}

impl JwtTokens {
    pub fn new(secret: &[u8], ttl_secs: u64) -> Self {
        Self {
            encoding: EncodingKey::from_secret(secret),
            decoding: DecodingKey::from_secret(secret),
            ttl_secs: ttl_secs as i64,
        }
    }
}

impl TokenService for JwtTokens {
    fn issue(&self, user: UserId) -> Result<String> {
        let now = chrono::Utc::now().timestamp();
        let claims = Claims {
            sub: user.0.to_string(),
            iat: now,
            exp: now + self.ttl_secs,
        };
        jsonwebtoken::encode(&Header::new(Algorithm::HS256), &claims, &self.encoding).map_err(infra)
    }

    fn verify(&self, token: &str) -> Result<UserId> {
        // Validation pins HS256 (no alg confusion) and checks exp.
        let data = jsonwebtoken::decode::<Claims>(
            token,
            &self.decoding,
            &Validation::new(Algorithm::HS256),
        )
        .map_err(|_| DomainError::Unauthorized)?;
        data.claims
            .sub
            .parse()
            .map(UserId)
            .map_err(|_| DomainError::Unauthorized)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn argon2_round_trip() {
        let hash = Argon2Hasher.hash("secret123").await.unwrap();
        assert!(hash.starts_with("$argon2id$"));
        assert!(Argon2Hasher.verify("secret123", &hash).await.unwrap());
        assert!(!Argon2Hasher.verify("wrong-pass", &hash).await.unwrap());
        assert!(Argon2Hasher.verify("x", "not-a-hash").await.is_err());
    }

    #[test]
    fn jwt_round_trip_and_rejections() {
        let tokens = JwtTokens::new(b"0123456789abcdef0123456789abcdef", 60);
        let user = UserId::generate();
        let token = tokens.issue(user).unwrap();
        assert_eq!(tokens.verify(&token), Ok(user));

        let other_key = JwtTokens::new(b"another-secret-another-secret-xx", 60);
        assert_eq!(other_key.verify(&token), Err(DomainError::Unauthorized));
        assert_eq!(tokens.verify("garbage"), Err(DomainError::Unauthorized));

        // exp well past the default 60s leeway.
        let expired = JwtTokens {
            ttl_secs: -120,
            ..JwtTokens::new(b"0123456789abcdef0123456789abcdef", 0)
        };
        assert_eq!(
            tokens.verify(&expired.issue(user).unwrap()),
            Err(DomainError::Unauthorized)
        );
    }
}
