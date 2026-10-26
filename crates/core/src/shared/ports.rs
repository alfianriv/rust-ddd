//! Ports used by more than one module. Implemented in `infra`.
use async_trait::async_trait;

use super::Result;
use crate::user::UserId;

#[async_trait]
pub trait PasswordHasher: Send + Sync {
    /// Async because real hashers (argon2) are CPU-heavy and run on a blocking pool.
    async fn hash(&self, password: &str) -> Result<String>;
    async fn verify(&self, password: &str, hash: &str) -> Result<bool>;
}

pub trait TokenService: Send + Sync {
    fn issue(&self, user: UserId) -> Result<String>;
    /// Invalid or expired token -> `DomainError::Unauthorized`.
    fn verify(&self, token: &str) -> Result<UserId>;
}
