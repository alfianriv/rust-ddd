use std::sync::Arc;

use super::domain::{Email, User, UserId, UserRepository};
use crate::shared::ports::{PasswordHasher, TokenService};
use crate::shared::{DomainError, Result};

fn validate_password(password: &str) -> Result<()> {
    // Upper bound stops someone making us hash a 10 MB string.
    match password.chars().count() {
        8..=128 => Ok(()),
        _ => Err(DomainError::Validation(
            "password must be 8-128 characters".into(),
        )),
    }
}

pub struct RegisterInput {
    pub email: String,
    pub name: String,
    pub password: String,
}

pub struct RegisterUser {
    users: Arc<dyn UserRepository>,
    hasher: Arc<dyn PasswordHasher>,
}

impl RegisterUser {
    pub fn new(users: Arc<dyn UserRepository>, hasher: Arc<dyn PasswordHasher>) -> Self {
        Self { users, hasher }
    }

    pub async fn execute(&self, input: RegisterInput) -> Result<User> {
        let email = Email::parse(&input.email)?;
        validate_password(&input.password)?;
        // Fast-path check; concurrent duplicates are caught by the DB unique index.
        if self.users.find_by_email(&email).await?.is_some() {
            return Err(DomainError::Conflict("email already registered".into()));
        }
        let hash = self.hasher.hash(&input.password).await?;
        let user = User::new(email, &input.name, hash)?;
        self.users.insert(&user).await?;
        Ok(user)
    }
}

pub struct LoginOutput {
    pub access_token: String,
    pub user: User,
}

pub struct Login {
    users: Arc<dyn UserRepository>,
    hasher: Arc<dyn PasswordHasher>,
    tokens: Arc<dyn TokenService>,
}

impl Login {
    pub fn new(
        users: Arc<dyn UserRepository>,
        hasher: Arc<dyn PasswordHasher>,
        tokens: Arc<dyn TokenService>,
    ) -> Self {
        Self {
            users,
            hasher,
            tokens,
        }
    }

    /// Every failure is `Unauthorized`, so callers can't tell "no such email" from "wrong password".
    pub async fn execute(&self, email: &str, password: &str) -> Result<LoginOutput> {
        let user = match Email::parse(email) {
            Ok(email) => self.users.find_by_email(&email).await?,
            Err(_) => None,
        };
        let Some(user) = user else {
            // Burn comparable time so response timing doesn't reveal which emails exist.
            self.hasher.hash(password).await?;
            return Err(DomainError::Unauthorized);
        };
        if !self.hasher.verify(password, &user.password_hash).await? {
            return Err(DomainError::Unauthorized);
        }
        Ok(LoginOutput {
            access_token: self.tokens.issue(user.id)?,
            user,
        })
    }
}

pub struct GetUser {
    users: Arc<dyn UserRepository>,
}

impl GetUser {
    pub fn new(users: Arc<dyn UserRepository>) -> Self {
        Self { users }
    }

    pub async fn execute(&self, id: UserId) -> Result<User> {
        self.users
            .find_by_id(id)
            .await?
            .ok_or(DomainError::NotFound("user"))
    }
}

pub struct UpdateProfile {
    users: Arc<dyn UserRepository>,
}

impl UpdateProfile {
    pub fn new(users: Arc<dyn UserRepository>) -> Self {
        Self { users }
    }

    pub async fn execute(&self, id: UserId, name: &str) -> Result<User> {
        let mut user = self
            .users
            .find_by_id(id)
            .await?
            .ok_or(DomainError::NotFound("user"))?;
        user.rename(name)?;
        self.users.update(&user).await?;
        Ok(user)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::{FakeHasher, FakeTokens, FakeUsers};

    fn input(email: &str, password: &str) -> RegisterInput {
        RegisterInput {
            email: email.into(),
            name: "Alfian".into(),
            password: password.into(),
        }
    }

    #[tokio::test]
    async fn register_then_login() {
        let users = Arc::new(FakeUsers::default());
        let hasher = Arc::new(FakeHasher);
        let register = RegisterUser::new(users.clone(), hasher.clone());
        let login = Login::new(users.clone(), hasher, Arc::new(FakeTokens));

        let user = register
            .execute(input(" A@B.com", "secret123"))
            .await
            .unwrap();
        assert_eq!(user.email.as_str(), "a@b.com");
        assert_ne!(user.password_hash, "secret123");

        let out = login.execute("a@b.com", "secret123").await.unwrap();
        assert_eq!(out.user.id, user.id);
        assert_eq!(out.access_token, format!("token:{}", user.id.0));
    }

    #[tokio::test]
    async fn register_rejects_duplicate_and_weak_password() {
        let users = Arc::new(FakeUsers::default());
        let register = RegisterUser::new(users, Arc::new(FakeHasher));
        register
            .execute(input("a@b.com", "secret123"))
            .await
            .unwrap();

        assert!(matches!(
            register.execute(input("A@b.com", "secret123")).await,
            Err(DomainError::Conflict(_))
        ));
        assert!(matches!(
            register.execute(input("c@d.com", "short")).await,
            Err(DomainError::Validation(_))
        ));
    }

    #[tokio::test]
    async fn login_failures_are_all_unauthorized() {
        let users = Arc::new(FakeUsers::default());
        let hasher = Arc::new(FakeHasher);
        RegisterUser::new(users.clone(), hasher.clone())
            .execute(input("a@b.com", "secret123"))
            .await
            .unwrap();
        let login = Login::new(users, hasher, Arc::new(FakeTokens));

        for (email, pw) in [
            ("a@b.com", "wrongpass"),
            ("x@y.com", "secret123"),
            ("bad", "x"),
        ] {
            assert_eq!(
                login.execute(email, pw).await.err(),
                Some(DomainError::Unauthorized)
            );
        }
    }

    #[tokio::test]
    async fn update_profile_validates_name() {
        let users = Arc::new(FakeUsers::default());
        let user = RegisterUser::new(users.clone(), Arc::new(FakeHasher))
            .execute(input("a@b.com", "secret123"))
            .await
            .unwrap();
        let update = UpdateProfile::new(users.clone());

        assert_eq!(
            update.execute(user.id, " Budi ").await.unwrap().name,
            "Budi"
        );
        assert!(update.execute(user.id, "  ").await.is_err());
        assert_eq!(
            GetUser::new(users).execute(UserId::generate()).await.err(),
            Some(DomainError::NotFound("user"))
        );
    }
}
