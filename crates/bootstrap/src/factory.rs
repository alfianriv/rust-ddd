//! Config -> adapter. The only place that knows which concrete adapter backs a port.
//! New adapter: add a variant to the config enum, add a match arm here.
use std::sync::Arc;

use anyhow::ensure;
use app_core::shared::ports::{PasswordHasher, TokenService};
use app_core::shipping_address::ShippingAddressRepository;
use app_core::user::UserRepository;
use infra::auth::{Argon2Hasher, JwtTokens};
use infra::persistence::{SeaOrmShippingAddressRepo, SeaOrmUserRepo};
use sea_orm::Database;

use crate::config::{DatabaseConfig, PasswordHasherConfig, TokenConfig};

/// Repositories share one connection pool, so they are built together.
pub struct Repositories {
    pub users: Arc<dyn UserRepository>,
    pub addresses: Arc<dyn ShippingAddressRepository>,
}

pub async fn repositories(cfg: &DatabaseConfig) -> anyhow::Result<Repositories> {
    match cfg {
        DatabaseConfig::Postgres { url } => {
            let db = Database::connect(url).await?;
            Ok(Repositories {
                users: Arc::new(SeaOrmUserRepo::new(db.clone())),
                addresses: Arc::new(SeaOrmShippingAddressRepo::new(db)),
            })
        }
    }
}

pub fn password_hasher(cfg: &PasswordHasherConfig) -> Arc<dyn PasswordHasher> {
    match cfg {
        PasswordHasherConfig::Argon2 => Arc::new(Argon2Hasher),
    }
}

pub fn token_service(cfg: &TokenConfig) -> anyhow::Result<Arc<dyn TokenService>> {
    match cfg {
        TokenConfig::Jwt { secret, ttl_secs } => {
            ensure!(
                secret.len() >= 32,
                "token.secret must be at least 32 bytes (set APP_TOKEN__SECRET)"
            );
            Ok(Arc::new(JwtTokens::new(secret.as_bytes(), *ttl_secs)))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_short_jwt_secret() {
        let short = TokenConfig::Jwt {
            secret: "too-short".into(),
            ttl_secs: 60,
        };
        assert!(token_service(&short).is_err());
    }
}
