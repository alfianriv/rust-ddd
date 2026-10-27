//! Runs against a real MySQL/MariaDB. Local: `TEST_DATABASE_URL=mysql://... cargo test -p infra`.
//! Skipped when the variable is unset, except in CI where it is required.
use std::sync::Arc;

use app_core::shared::DomainError;
use app_core::shipping_address::{
    AddAddress, AddressDetails, AddressId, DeleteAddress, ShippingAddress,
    ShippingAddressRepository, UpdateAddress,
};
use app_core::user::{Email, User, UserId, UserRepository};
use infra::persistence::{SeaOrmShippingAddressRepo, SeaOrmUserRepo};
use migration::{Migrator, MigratorTrait};
use sea_orm::{Database, DatabaseConnection};

#[tokio::test]
async fn repositories_against_real_db() {
    let Ok(url) = std::env::var("TEST_DATABASE_URL") else {
        assert!(
            std::env::var_os("CI").is_none(),
            "TEST_DATABASE_URL must be set in CI"
        );
        eprintln!("skipped: set TEST_DATABASE_URL to run DB tests");
        return;
    };
    let db = Database::connect(&url).await.unwrap();
    Migrator::up(&db, None).await.unwrap();

    users(&db).await;
    addresses(&db).await;
}

/// Tests share one DB without cleanup, so every user gets a fresh email.
fn new_user(name: &str) -> User {
    let email = Email::parse(&format!("{}@test.local", UserId::generate().0.simple())).unwrap();
    User::new(email, name, "hash".into()).unwrap()
}

async fn users(db: &DatabaseConnection) {
    let repo = SeaOrmUserRepo::new(db.clone());
    let mut user = new_user("Alfian");
    repo.insert(&user).await.unwrap();

    let found = repo.find_by_email(&user.email).await.unwrap().unwrap();
    assert_eq!((found.id, found.name.as_str()), (user.id, "Alfian"));

    let dup = User::new(user.email.clone(), "Other", "h".into()).unwrap();
    assert_eq!(
        repo.insert(&dup).await,
        Err(DomainError::Conflict("email already registered".into()))
    );

    user.rename("Budi").unwrap();
    repo.update(&user).await.unwrap();
    repo.update(&user).await.unwrap(); // no-op update must not look like "not found"
    assert_eq!(
        repo.find_by_id(user.id).await.unwrap().unwrap().name,
        "Budi"
    );

    assert_eq!(
        repo.update(&new_user("ghost")).await,
        Err(DomainError::NotFound("user"))
    );
}

fn details(label: &str) -> AddressDetails {
    AddressDetails {
        label: label.into(),
        recipient_name: "Alfian".into(),
        phone: "+6281234567890".into(),
        address_line: "Jl. Pajajaran No. 1".into(),
        city: "Bogor".into(),
        province: "Jawa Barat".into(),
        postal_code: "16143".into(),
        notes: Some("pagar hitam".into()),
    }
}

async fn labels_and_default(
    repo: &dyn ShippingAddressRepository,
    user: UserId,
) -> (Vec<String>, String) {
    let list = repo.list_by_user(user).await.unwrap();
    let defaults: Vec<_> = list.iter().filter(|a| a.is_default).collect();
    assert_eq!(defaults.len(), 1, "exactly one default");
    let default = defaults[0].details.label.clone();
    (list.into_iter().map(|a| a.details.label).collect(), default)
}

async fn addresses(db: &DatabaseConnection) {
    let users = SeaOrmUserRepo::new(db.clone());
    let (owner, other) = (new_user("owner"), new_user("other"));
    users.insert(&owner).await.unwrap();
    users.insert(&other).await.unwrap();

    let repo: Arc<dyn ShippingAddressRepository> =
        Arc::new(SeaOrmShippingAddressRepo::new(db.clone()));
    let add = AddAddress::new(repo.clone());

    let a = add.execute(owner.id, details("A"), false).await.unwrap();
    let b = add.execute(owner.id, details("B"), false).await.unwrap();
    let c = add.execute(owner.id, details("C"), true).await.unwrap();
    assert!(a.is_default && !b.is_default && c.is_default);
    assert_eq!(
        labels_and_default(repo.as_ref(), owner.id).await,
        (vec!["C".into(), "B".into(), "A".into()], "C".into())
    );

    // DB refuses a second default even if app code is bypassed.
    let rogue = ShippingAddress::new(owner.id, details("R"), true).unwrap();
    assert!(matches!(
        repo.insert(&rogue).await,
        Err(DomainError::Conflict(_))
    ));

    // Update rewrites details only, scoped to the owner.
    let update = UpdateAddress::new(repo.clone());
    let a2 = update.execute(owner.id, a.id, details("A2")).await.unwrap();
    repo.update(&a2).await.unwrap(); // no-op update
    assert_eq!(
        update.execute(other.id, a.id, details("hack")).await.err(),
        Some(DomainError::Forbidden)
    );
    let stored = repo.find_by_id(a.id).await.unwrap().unwrap();
    assert_eq!(stored.details.label, "A2");
    assert_eq!(stored.details.notes.as_deref(), Some("pagar hitam"));
    assert!(!stored.is_default);
    assert!(
        repo.find_by_id(AddressId::generate())
            .await
            .unwrap()
            .is_none()
    );
    // Repo writes stay scoped by user even if a caller skips the ownership check.
    let mut hijack = stored.clone();
    hijack.user_id = other.id;
    assert_eq!(
        repo.update(&hijack).await,
        Err(DomainError::NotFound("shipping address"))
    );

    // Failed set_default rolls back the clear.
    assert_eq!(
        repo.set_default(owner.id, AddressId::generate()).await,
        Err(DomainError::NotFound("shipping address"))
    );
    assert_eq!(labels_and_default(repo.as_ref(), owner.id).await.1, "C");

    // Deleting the default promotes the newest remaining one.
    DeleteAddress::new(repo.clone())
        .execute(owner.id, c.id)
        .await
        .unwrap();
    assert_eq!(
        labels_and_default(repo.as_ref(), owner.id).await,
        (vec!["B".into(), "A2".into()], "B".into())
    );

    // Another user's delete is a no-op.
    repo.delete(other.id, b.id).await.unwrap();
    assert_eq!(repo.count_by_user(owner.id).await.unwrap(), 2);
    assert_eq!(repo.count_by_user(other.id).await.unwrap(), 0);
}
