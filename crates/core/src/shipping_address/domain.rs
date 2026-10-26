use async_trait::async_trait;
use chrono::{DateTime, Utc};
use uuid::Uuid;

use crate::shared::{DomainError, Result, required_text};
use crate::user::UserId;

pub const MAX_ADDRESSES_PER_USER: usize = 10;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct AddressId(pub Uuid);

impl AddressId {
    pub fn generate() -> Self {
        Self(Uuid::now_v7())
    }
}

/// The editable part of an address. Always go through `validated()`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AddressDetails {
    pub label: String,
    pub recipient_name: String,
    pub phone: String,
    pub address_line: String,
    pub city: String,
    pub province: String,
    pub postal_code: String,
    pub notes: Option<String>,
}

impl AddressDetails {
    /// Returns a trimmed/normalized copy, or the first validation error.
    pub fn validated(self) -> Result<Self> {
        let postal_code = self.postal_code.trim().to_string();
        if postal_code.len() != 5 || !postal_code.bytes().all(|b| b.is_ascii_digit()) {
            return Err(DomainError::Validation(
                "postal_code must be 5 digits".into(),
            ));
        }
        let notes = match self.notes.as_deref().map(str::trim) {
            None | Some("") => None,
            Some(n) if n.chars().count() <= 255 => Some(n.to_string()),
            Some(_) => {
                return Err(DomainError::Validation(
                    "notes must be at most 255 characters".into(),
                ));
            }
        };
        Ok(Self {
            label: required_text("label", &self.label, 30)?,
            recipient_name: required_text("recipient_name", &self.recipient_name, 100)?,
            phone: normalize_phone(&self.phone)?,
            address_line: required_text("address_line", &self.address_line, 255)?,
            city: required_text("city", &self.city, 100)?,
            province: required_text("province", &self.province, 100)?,
            postal_code,
            notes,
        })
    }
}

/// Drops spaces/dashes, keeps an optional leading '+'. 8-15 digits (E.164 max).
fn normalize_phone(raw: &str) -> Result<String> {
    let phone: String = raw
        .trim()
        .chars()
        .filter(|c| !matches!(c, ' ' | '-'))
        .collect();
    let digits = phone.strip_prefix('+').unwrap_or(&phone);
    if (8..=15).contains(&digits.len()) && digits.bytes().all(|b| b.is_ascii_digit()) {
        Ok(phone)
    } else {
        Err(DomainError::Validation("phone must be 8-15 digits".into()))
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct ShippingAddress {
    pub id: AddressId,
    pub user_id: UserId,
    pub details: AddressDetails,
    pub is_default: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl ShippingAddress {
    pub fn new(user_id: UserId, details: AddressDetails, is_default: bool) -> Result<Self> {
        let now = Utc::now();
        Ok(Self {
            id: AddressId::generate(),
            user_id,
            details: details.validated()?,
            is_default,
            created_at: now,
            updated_at: now,
        })
    }

    pub fn update_details(&mut self, details: AddressDetails) -> Result<()> {
        self.details = details.validated()?;
        self.updated_at = Utc::now();
        Ok(())
    }
}

/// Every method is scoped by `user` so one user can never touch another's addresses.
#[async_trait]
pub trait ShippingAddressRepository: Send + Sync {
    /// Ordered: default first, then newest first.
    async fn list_by_user(&self, user: UserId) -> Result<Vec<ShippingAddress>>;
    async fn find_for_user(&self, user: UserId, id: AddressId) -> Result<Option<ShippingAddress>>;
    async fn count_by_user(&self, user: UserId) -> Result<usize>;
    async fn insert(&self, address: &ShippingAddress) -> Result<()>;
    async fn update(&self, address: &ShippingAddress) -> Result<()>;
    async fn delete(&self, user: UserId, id: AddressId) -> Result<()>;
    /// Atomic (one DB transaction): unset the current default, set `id` as default.
    async fn set_default(&self, user: UserId, id: AddressId) -> Result<()>;
}
