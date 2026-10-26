//! Real adapters end to end: Postgres + argon2 + JWT. Same skip rule as infra's DB test.
use app_core::shipping_address::AddressDetails;
use app_core::user::{RegisterInput, UserId};
use bootstrap::{Config, Container};
use migration::{Migrator, MigratorTrait};

#[tokio::test]
async fn register_login_and_address_flow() {
    let Ok(url) = std::env::var("TEST_DATABASE_URL") else {
        assert!(
            std::env::var_os("CI").is_none(),
            "TEST_DATABASE_URL must be set in CI"
        );
        eprintln!("skipped: set TEST_DATABASE_URL to run DB tests");
        return;
    };
    let cfg = Config {
        database_url: url,
        jwt_secret: "test-secret-test-secret-test-secret".into(),
        jwt_ttl_secs: 60,
    };
    let c = Container::new(&cfg).await.unwrap();
    let db = sea_orm::Database::connect(&cfg.database_url).await.unwrap();
    Migrator::up(&db, None).await.unwrap();

    let email = format!("{}@test.local", UserId::generate().0.simple());
    let user = c
        .register_user
        .execute(RegisterInput {
            email: email.clone(),
            name: "Alfian".into(),
            password: "secret123".into(),
        })
        .await
        .unwrap();

    let out = c.login.execute(&email, "secret123").await.unwrap();
    assert_eq!(c.tokens.verify(&out.access_token), Ok(user.id));
    assert!(c.login.execute(&email, "wrong-pass").await.is_err());

    let details = AddressDetails {
        label: "Rumah".into(),
        recipient_name: "Alfian".into(),
        phone: "081234567890".into(),
        address_line: "Jl. Pajajaran No. 1".into(),
        city: "Bogor".into(),
        province: "Jawa Barat".into(),
        postal_code: "16143".into(),
        notes: None,
    };
    c.add_address
        .execute(user.id, details, false)
        .await
        .unwrap();
    let list = c.list_addresses.execute(user.id).await.unwrap();
    assert_eq!(list.len(), 1);
    assert!(list[0].is_default);
}
