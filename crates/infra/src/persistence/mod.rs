mod entities;
mod shipping_address_repo;
mod user_repo;

use app_core::shared::DomainError;
use sea_orm::{DbErr, SqlErr};

pub use shipping_address_repo::SeaOrmShippingAddressRepo;
pub use user_repo::SeaOrmUserRepo;

/// Unique violations become `Conflict` with a client-safe message; everything else is `Infra`.
fn db_err(e: DbErr, conflict_msg: &str) -> DomainError {
    match e.sql_err() {
        Some(SqlErr::UniqueConstraintViolation(_)) => DomainError::Conflict(conflict_msg.into()),
        _ => DomainError::Infra(e.to_string()),
    }
}

fn infra(e: DbErr) -> DomainError {
    DomainError::Infra(e.to_string())
}
