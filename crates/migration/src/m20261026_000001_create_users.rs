use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, m: &SchemaManager) -> Result<(), DbErr> {
        m.create_table(
            Table::create()
                .table(Users::Table)
                .col(ColumnDef::new(Users::Id).uuid().not_null().primary_key())
                .col(
                    ColumnDef::new(Users::Email)
                        .string_len(254)
                        .not_null()
                        .unique_key(),
                )
                .col(ColumnDef::new(Users::Name).string_len(100).not_null())
                .col(
                    ColumnDef::new(Users::PasswordHash)
                        .string_len(255)
                        .not_null(),
                )
                .col(ColumnDef::new(Users::CreatedAt).date_time().not_null())
                .col(ColumnDef::new(Users::UpdatedAt).date_time().not_null())
                .to_owned(),
        )
        .await
    }

    async fn down(&self, m: &SchemaManager) -> Result<(), DbErr> {
        m.drop_table(Table::drop().table(Users::Table).to_owned())
            .await
    }
}

#[derive(DeriveIden)]
pub enum Users {
    Table,
    Id,
    Email,
    Name,
    PasswordHash,
    CreatedAt,
    UpdatedAt,
}
