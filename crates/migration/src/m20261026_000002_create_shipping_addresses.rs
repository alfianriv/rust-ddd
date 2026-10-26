use sea_orm_migration::prelude::*;

/// Convention: no FOREIGN KEY constraints in the schema. Relations live in code
/// (SeaORM entity relations + use cases); `user_id` only gets an index.
#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, m: &SchemaManager) -> Result<(), DbErr> {
        m.create_table(
            Table::create()
                .table(A::Table)
                .col(ColumnDef::new(A::Id).uuid().not_null().primary_key())
                .col(ColumnDef::new(A::UserId).uuid().not_null())
                .col(ColumnDef::new(A::Label).string_len(30).not_null())
                .col(ColumnDef::new(A::RecipientName).string_len(100).not_null())
                .col(ColumnDef::new(A::Phone).string_len(16).not_null())
                .col(ColumnDef::new(A::AddressLine).string_len(255).not_null())
                .col(ColumnDef::new(A::City).string_len(100).not_null())
                .col(ColumnDef::new(A::Province).string_len(100).not_null())
                .col(ColumnDef::new(A::PostalCode).string_len(5).not_null())
                .col(ColumnDef::new(A::Notes).string_len(255).null())
                .col(ColumnDef::new(A::IsDefault).boolean().not_null())
                .col(
                    ColumnDef::new(A::CreatedAt)
                        .timestamp_with_time_zone()
                        .not_null(),
                )
                .col(
                    ColumnDef::new(A::UpdatedAt)
                        .timestamp_with_time_zone()
                        .not_null(),
                )
                .to_owned(),
        )
        .await?;

        m.create_index(
            Index::create()
                .name("idx_shipping_addresses_user_id")
                .table(A::Table)
                .col(A::UserId)
                .to_owned(),
        )
        .await?;

        // At most one default per user, enforced by the DB (Postgres partial unique index).
        m.get_connection()
            .execute_unprepared(
                "CREATE UNIQUE INDEX uq_shipping_addresses_one_default \
                 ON shipping_addresses (user_id) WHERE is_default",
            )
            .await?;
        Ok(())
    }

    async fn down(&self, m: &SchemaManager) -> Result<(), DbErr> {
        m.drop_table(Table::drop().table(A::Table).to_owned()).await
    }
}

use ShippingAddresses as A;

#[derive(DeriveIden)]
enum ShippingAddresses {
    Table,
    Id,
    UserId,
    Label,
    RecipientName,
    Phone,
    AddressLine,
    City,
    Province,
    PostalCode,
    Notes,
    IsDefault,
    CreatedAt,
    UpdatedAt,
}
