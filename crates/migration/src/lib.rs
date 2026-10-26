pub use sea_orm_migration::prelude::*;

mod m20261026_000001_create_users;
mod m20261026_000002_create_shipping_addresses;

pub struct Migrator;

#[async_trait::async_trait]
impl MigratorTrait for Migrator {
    fn migrations() -> Vec<Box<dyn MigrationTrait>> {
        vec![
            Box::new(m20261026_000001_create_users::Migration),
            Box::new(m20261026_000002_create_shipping_addresses::Migration),
        ]
    }
}
