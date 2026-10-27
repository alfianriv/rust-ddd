use std::sync::Arc;

use app_core::shared::ports::TokenService;
use app_core::shipping_address::{
    AddAddress, DeleteAddress, ListAddresses, SetDefaultAddress, UpdateAddress,
};
use app_core::user::{GetUser, Login, RegisterUser, UpdateProfile};

use crate::{Config, factory};

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
        // Fail fast on bad config before opening DB connections.
        let tokens = factory::token_service(&cfg.token)?;
        let hasher = factory::password_hasher(&cfg.password_hasher);
        let repos = factory::repositories(&cfg.database).await?;
        let (users, addresses) = (repos.users, repos.addresses);

        Ok(Self {
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
        })
    }
}
