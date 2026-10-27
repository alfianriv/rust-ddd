use app_core::shared::{DomainError, Result};
use app_core::shipping_address::{
    AddressDetails, AddressId, ShippingAddress, ShippingAddressRepository,
};
use app_core::user::UserId;
use async_trait::async_trait;
use sea_orm::sea_query::Expr;
use sea_orm::{
    ActiveModelTrait, ActiveValue::Set, ColumnTrait, DatabaseConnection, EntityTrait,
    IntoActiveModel, PaginatorTrait, QueryFilter, QueryOrder, TransactionTrait,
};

use super::entities::shipping_addresses::{self as addr, Column};
use super::{db_err, infra};

const DEFAULT_CONFLICT: &str = "default address changed concurrently, retry";

pub struct SeaOrmShippingAddressRepo {
    db: DatabaseConnection,
}

impl SeaOrmShippingAddressRepo {
    pub fn new(db: DatabaseConnection) -> Self {
        Self { db }
    }
}

fn to_domain(m: addr::Model) -> ShippingAddress {
    ShippingAddress {
        id: AddressId(m.id),
        user_id: UserId(m.user_id),
        details: AddressDetails {
            label: m.label,
            recipient_name: m.recipient_name,
            phone: m.phone,
            address_line: m.address_line,
            city: m.city,
            province: m.province,
            postal_code: m.postal_code,
            notes: m.notes,
        },
        is_default: m.is_default,
        created_at: m.created_at,
        updated_at: m.updated_at,
    }
}

fn owned(user: UserId, id: AddressId) -> sea_orm::Condition {
    sea_orm::Condition::all()
        .add(Column::Id.eq(id.0))
        .add(Column::UserId.eq(user.0))
}

#[async_trait]
impl ShippingAddressRepository for SeaOrmShippingAddressRepo {
    async fn list_by_user(&self, user: UserId) -> Result<Vec<ShippingAddress>> {
        // UUIDv7 bytes sort by creation time, so `id DESC` = newest first.
        let rows = addr::Entity::find()
            .filter(Column::UserId.eq(user.0))
            .order_by_desc(Column::IsDefault)
            .order_by_desc(Column::Id)
            .all(&self.db)
            .await
            .map_err(infra)?;
        Ok(rows.into_iter().map(to_domain).collect())
    }

    async fn find_by_id(&self, id: AddressId) -> Result<Option<ShippingAddress>> {
        Ok(addr::Entity::find_by_id(id.0)
            .one(&self.db)
            .await
            .map_err(infra)?
            .map(to_domain))
    }

    async fn count_by_user(&self, user: UserId) -> Result<usize> {
        let n = addr::Entity::find()
            .filter(Column::UserId.eq(user.0))
            .count(&self.db)
            .await
            .map_err(infra)?;
        Ok(n as usize)
    }

    async fn insert(&self, a: &ShippingAddress) -> Result<()> {
        let d = &a.details;
        let model = addr::Model {
            id: a.id.0,
            user_id: a.user_id.0,
            label: d.label.clone(),
            recipient_name: d.recipient_name.clone(),
            phone: d.phone.clone(),
            address_line: d.address_line.clone(),
            city: d.city.clone(),
            province: d.province.clone(),
            postal_code: d.postal_code.clone(),
            notes: d.notes.clone(),
            is_default: a.is_default,
            created_at: a.created_at,
            updated_at: a.updated_at,
        };
        addr::Entity::insert(model.into_active_model().reset_all())
            .exec_without_returning(&self.db)
            .await
            .map_err(|e| db_err(e, DEFAULT_CONFLICT))?;
        Ok(())
    }

    /// Writes details + updated_at only. `is_default` is owned by `set_default`,
    /// so a stale copy can never overwrite a concurrent default switch.
    async fn update(&self, a: &ShippingAddress) -> Result<()> {
        let d = &a.details;
        let changes = addr::ActiveModel {
            label: Set(d.label.clone()),
            recipient_name: Set(d.recipient_name.clone()),
            phone: Set(d.phone.clone()),
            address_line: Set(d.address_line.clone()),
            city: Set(d.city.clone()),
            province: Set(d.province.clone()),
            postal_code: Set(d.postal_code.clone()),
            notes: Set(d.notes.clone()),
            updated_at: Set(a.updated_at),
            ..Default::default()
        };
        let res = addr::Entity::update_many()
            .set(changes)
            .filter(owned(a.user_id, a.id))
            .exec(&self.db)
            .await
            .map_err(infra)?;
        if res.rows_affected == 0 {
            return Err(DomainError::NotFound("shipping address"));
        }
        Ok(())
    }

    async fn delete(&self, user: UserId, id: AddressId) -> Result<()> {
        addr::Entity::delete_many()
            .filter(owned(user, id))
            .exec(&self.db)
            .await
            .map_err(infra)?;
        Ok(())
    }

    async fn set_default(&self, user: UserId, id: AddressId) -> Result<()> {
        let txn = self.db.begin().await.map_err(infra)?;
        // Clear first: the unique index allows only one default per user at any moment.
        addr::Entity::update_many()
            .col_expr(Column::IsDefault, Expr::value(false))
            .filter(Column::UserId.eq(user.0))
            .filter(Column::IsDefault.eq(true))
            .exec(&txn)
            .await
            .map_err(|e| db_err(e, DEFAULT_CONFLICT))?;
        let res = addr::Entity::update_many()
            .col_expr(Column::IsDefault, Expr::value(true))
            .filter(owned(user, id))
            .exec(&txn)
            .await
            .map_err(|e| db_err(e, DEFAULT_CONFLICT))?;
        if res.rows_affected == 0 {
            // Dropping `txn` rolls back the clear above.
            return Err(DomainError::NotFound("shipping address"));
        }
        txn.commit().await.map_err(infra)
    }
}
