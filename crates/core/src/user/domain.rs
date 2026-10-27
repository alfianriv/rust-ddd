use async_trait::async_trait;
use chrono::{DateTime, Utc};
use uuid::Uuid;

use crate::shared::{DomainError, Result, required_text};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct UserId(pub Uuid);

impl UserId {
    pub fn generate() -> Self {
        Self(Uuid::now_v7())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Email(String);

impl Email {
    /// Trims + lowercases. Shape check only; real ownership proof = verification email.
    pub fn parse(raw: &str) -> Result<Self> {
        let e = raw.trim().to_lowercase();
        let valid = e.len() <= 254
            && !e.contains(char::is_whitespace)
            && match e.split_once('@') {
                Some((local, domain)) => {
                    !local.is_empty()
                        && !domain.contains('@')
                        && domain.contains('.')
                        && !domain.starts_with('.')
                        && !domain.ends_with('.')
                }
                None => false,
            };
        if !valid {
            return Err(DomainError::Validation("invalid email".into()));
        }
        Ok(Self(e))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// Fields are pub so infra can rehydrate from DB rows.
/// Use `User::new` / methods for changes so validation always runs.
#[derive(Debug, Clone, PartialEq)]
pub struct User {
    pub id: UserId,
    pub email: Email,
    pub name: String,
    pub password_hash: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl User {
    pub fn new(email: Email, name: &str, password_hash: String) -> Result<Self> {
        let now = crate::shared::now();
        Ok(Self {
            id: UserId::generate(),
            email,
            name: required_text("name", name, 100)?,
            password_hash,
            created_at: now,
            updated_at: now,
        })
    }

    pub fn rename(&mut self, name: &str) -> Result<()> {
        self.name = required_text("name", name, 100)?;
        self.updated_at = crate::shared::now();
        Ok(())
    }
}

#[async_trait]
pub trait UserRepository: Send + Sync {
    async fn find_by_id(&self, id: UserId) -> Result<Option<User>>;
    async fn find_by_email(&self, email: &Email) -> Result<Option<User>>;
    /// Duplicate email (unique index) must map to `DomainError::Conflict`.
    async fn insert(&self, user: &User) -> Result<()>;
    async fn update(&self, user: &User) -> Result<()>;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn email_normalized_and_validated() {
        assert_eq!(
            Email::parse("  Foo@Bar.com ").unwrap().as_str(),
            "foo@bar.com"
        );
        for bad in [
            "",
            "foo",
            "@bar.com",
            "foo@bar",
            "foo@.com",
            "a b@c.com",
            "a@b@c.com",
        ] {
            assert!(Email::parse(bad).is_err(), "{bad:?} should be rejected");
        }
    }
}
