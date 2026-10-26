use std::sync::Arc;

use app_core::shared::ports::{PasswordHasher, TokenService};
use app_core::shipping_address::{
    AddAddress, DeleteAddress, ListAddresses, SetDefaultAddress, ShippingAddressRepository,
    UpdateAddress,
};
use app_core::user::{GetUser, Login, RegisterUser, UpdateProfile, UserRepository};
use infra::auth::{Argon2Hasher, JwtTokens};
use infra::persistence::{SeaOrmShippingAddressRepo, SeaOrmUserRepo};
use sea_orm::{Database, DatabaseConnection};

use crate::Config;

/// Every use case, wired once. Apps hold it in an `Arc` and only call `.execute()`.
pub struct Container {
    /// Exposed for auth middleware (verifying bearer tokens).
    pub tokens: Arc<dyn TokenService>,

    pub register_user: RegisterUser,
    pub login: Login,
    pub get_user: GetUser,
    pub update_profile: UpdateProfile,

    pub list_addresses: ListAddresses,
    pub add_address: AddAddress,
    pub update_address: UpdateAddress,
    pub delete_address: DeleteAddress,
    pub set_default_address: SetDefaultAddress,
}

impl Container {
    pub async fn new(cfg: &Config) -> anyhow::Result<Self> {
        let db = Database::connect(&cfg.database_url).await?;
        Ok(Self::with_db(db, cfg))
    }

    pub fn with_db(db: DatabaseConnection, cfg: &Config) -> Self {
        let users: Arc<dyn UserRepository> = Arc::new(SeaOrmUserRepo::new(db.clone()));
        let addresses: Arc<dyn ShippingAddressRepository> =
            Arc::new(SeaOrmShippingAddressRepo::new(db));
        let hasher: Arc<dyn PasswordHasher> = Arc::new(Argon2Hasher);
        let tokens: Arc<dyn TokenService> =
            Arc::new(JwtTokens::new(cfg.jwt_secret.as_bytes(), cfg.jwt_ttl_secs));

        Self {
            register_user: RegisterUser::new(users.clone(), hasher.clone()),
            login: Login::new(users.clone(), hasher, tokens.clone()),
            get_user: GetUser::new(users.clone()),
            update_profile: UpdateProfile::new(users),

            list_addresses: ListAddresses::new(addresses.clone()),
            add_address: AddAddress::new(addresses.clone()),
            update_address: UpdateAddress::new(addresses.clone()),
            delete_address: DeleteAddress::new(addresses.clone()),
            set_default_address: SetDefaultAddress::new(addresses),

            tokens,
        }
    }
}
