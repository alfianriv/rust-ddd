//! Table mappings. Must match crates/migration.
//! Relations are declared here only; the schema has no FOREIGN KEY constraints.

pub mod users {
    use sea_orm::entity::prelude::*;

    #[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel)]
    #[sea_orm(table_name = "users")]
    pub struct Model {
        #[sea_orm(primary_key, auto_increment = false)]
        pub id: Uuid,
        pub email: String,
        pub name: String,
        pub password_hash: String,
        pub created_at: DateTimeUtc,
        pub updated_at: DateTimeUtc,
    }

    #[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
    pub enum Relation {
        #[sea_orm(has_many = "super::shipping_addresses::Entity")]
        ShippingAddresses,
    }

    impl Related<super::shipping_addresses::Entity> for Entity {
        fn to() -> RelationDef {
            Relation::ShippingAddresses.def()
        }
    }

    impl ActiveModelBehavior for ActiveModel {}
}

pub mod shipping_addresses {
    use sea_orm::entity::prelude::*;

    #[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel)]
    #[sea_orm(table_name = "shipping_addresses")]
    pub struct Model {
        #[sea_orm(primary_key, auto_increment = false)]
        pub id: Uuid,
        pub user_id: Uuid,
        pub label: String,
        pub recipient_name: String,
        pub phone: String,
        pub address_line: String,
        pub city: String,
        pub province: String,
        pub postal_code: String,
        pub notes: Option<String>,
        pub is_default: bool,
        pub created_at: DateTimeUtc,
        pub updated_at: DateTimeUtc,
    }

    #[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
    pub enum Relation {
        #[sea_orm(
            belongs_to = "super::users::Entity",
            from = "Column::UserId",
            to = "super::users::Column::Id"
        )]
        User,
    }

    impl Related<super::users::Entity> for Entity {
        fn to() -> RelationDef {
            Relation::User.def()
        }
    }

    impl ActiveModelBehavior for ActiveModel {}
}
