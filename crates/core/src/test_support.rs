//! In-memory fakes for use-case tests.
use std::sync::Mutex;

use async_trait::async_trait;

use crate::shared::ports::{PasswordHasher, TokenService};
use crate::shared::{DomainError, Result};
use crate::shipping_address::{AddressId, ShippingAddress, ShippingAddressRepository};
use crate::user::{Email, User, UserId, UserRepository};

#[derive(Default)]
pub struct FakeUsers(Mutex<Vec<User>>);

#[async_trait]
impl UserRepository for FakeUsers {
    async fn find_by_id(&self, id: UserId) -> Result<Option<User>> {
        Ok(self.0.lock().unwrap().iter().find(|u| u.id == id).cloned())
    }
    async fn find_by_email(&self, email: &Email) -> Result<Option<User>> {
        Ok(self
            .0
            .lock()
            .unwrap()
            .iter()
            .find(|u| &u.email == email)
            .cloned())
    }
    async fn insert(&self, user: &User) -> Result<()> {
        self.0.lock().unwrap().push(user.clone());
        Ok(())
    }
    async fn update(&self, user: &User) -> Result<()> {
        let mut users = self.0.lock().unwrap();
        let slot = users.iter_mut().find(|u| u.id == user.id);
        *slot.ok_or(DomainError::NotFound("user"))? = user.clone();
        Ok(())
    }
}

pub struct FakeHasher;

#[async_trait]
impl PasswordHasher for FakeHasher {
    async fn hash(&self, password: &str) -> Result<String> {
        Ok(format!("hashed:{password}"))
    }
    async fn verify(&self, password: &str, hash: &str) -> Result<bool> {
        Ok(hash == format!("hashed:{password}"))
    }
}

pub struct FakeTokens;

impl TokenService for FakeTokens {
    fn issue(&self, user: UserId) -> Result<String> {
        Ok(format!("token:{}", user.0))
    }
    fn verify(&self, token: &str) -> Result<UserId> {
        token
            .strip_prefix("token:")
            .and_then(|id| id.parse().ok())
            .map(UserId)
            .ok_or(DomainError::Unauthorized)
    }
}

#[derive(Default)]
pub struct FakeAddresses(Mutex<Vec<ShippingAddress>>);

#[async_trait]
impl ShippingAddressRepository for FakeAddresses {
    async fn list_by_user(&self, user: UserId) -> Result<Vec<ShippingAddress>> {
        let mut list: Vec<_> = self
            .0
            .lock()
            .unwrap()
            .iter()
            .filter(|a| a.user_id == user)
            .cloned()
            .collect();
        // Same order the real repo must return. v7 ids are time-ordered.
        list.sort_by_key(|a| std::cmp::Reverse((a.is_default, a.id.0)));
        Ok(list)
    }
    async fn find_by_id(&self, id: AddressId) -> Result<Option<ShippingAddress>> {
        Ok(self.0.lock().unwrap().iter().find(|a| a.id == id).cloned())
    }
    async fn count_by_user(&self, user: UserId) -> Result<usize> {
        Ok(self
            .0
            .lock()
            .unwrap()
            .iter()
            .filter(|a| a.user_id == user)
            .count())
    }
    async fn insert(&self, address: &ShippingAddress) -> Result<()> {
        self.0.lock().unwrap().push(address.clone());
        Ok(())
    }
    async fn update(&self, address: &ShippingAddress) -> Result<()> {
        let mut all = self.0.lock().unwrap();
        let slot = all.iter_mut().find(|a| a.id == address.id);
        *slot.ok_or(DomainError::NotFound("shipping address"))? = address.clone();
        Ok(())
    }
    async fn delete(&self, user: UserId, id: AddressId) -> Result<()> {
        self.0
            .lock()
            .unwrap()
            .retain(|a| !(a.user_id == user && a.id == id));
        Ok(())
    }
    async fn set_default(&self, user: UserId, id: AddressId) -> Result<()> {
        for a in self
            .0
            .lock()
            .unwrap()
            .iter_mut()
            .filter(|a| a.user_id == user)
        {
            a.is_default = a.id == id;
        }
        Ok(())
    }
}
