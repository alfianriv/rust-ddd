use app_core::shared::{DomainError, Result};
use app_core::user::{Email, User, UserId, UserRepository};
use async_trait::async_trait;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, DatabaseConnection, DbErr, EntityTrait, IntoActiveModel,
    QueryFilter,
};

use super::entities::users;
use super::{db_err, infra};

pub struct SeaOrmUserRepo {
    db: DatabaseConnection,
}

impl SeaOrmUserRepo {
    pub fn new(db: DatabaseConnection) -> Self {
        Self { db }
    }
}

fn to_domain(m: users::Model) -> Result<User> {
    let email = Email::parse(&m.email)
        .map_err(|_| DomainError::Infra(format!("invalid email stored for user {}", m.id)))?;
    Ok(User {
        id: UserId(m.id),
        email,
        name: m.name,
        password_hash: m.password_hash,
        created_at: m.created_at,
        updated_at: m.updated_at,
    })
}

/// Every column `Set`, ready for insert/update.
fn to_active(u: &User) -> users::ActiveModel {
    users::Model {
        id: u.id.0,
        email: u.email.as_str().to_string(),
        name: u.name.clone(),
        password_hash: u.password_hash.clone(),
        created_at: u.created_at,
        updated_at: u.updated_at,
    }
    .into_active_model()
    .reset_all()
}

#[async_trait]
impl UserRepository for SeaOrmUserRepo {
    async fn find_by_id(&self, id: UserId) -> Result<Option<User>> {
        users::Entity::find_by_id(id.0)
            .one(&self.db)
            .await
            .map_err(infra)?
            .map(to_domain)
            .transpose()
    }

    async fn find_by_email(&self, email: &Email) -> Result<Option<User>> {
        users::Entity::find()
            .filter(users::Column::Email.eq(email.as_str()))
            .one(&self.db)
            .await
            .map_err(infra)?
            .map(to_domain)
            .transpose()
    }

    async fn insert(&self, user: &User) -> Result<()> {
        users::Entity::insert(to_active(user))
            .exec_without_returning(&self.db)
            .await
            .map_err(|e| db_err(e, "email already registered"))?;
        Ok(())
    }

    async fn update(&self, user: &User) -> Result<()> {
        match to_active(user).update(&self.db).await {
            Ok(_) => Ok(()),
            Err(DbErr::RecordNotUpdated) => Err(DomainError::NotFound("user")),
            Err(e) => Err(db_err(e, "email already registered")),
        }
    }
}
