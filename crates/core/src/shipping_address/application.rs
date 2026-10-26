use std::sync::Arc;

use super::domain::{
    AddressDetails, AddressId, MAX_ADDRESSES_PER_USER, ShippingAddress, ShippingAddressRepository,
};
use crate::shared::{DomainError, Result};
use crate::user::UserId;

type Repo = Arc<dyn ShippingAddressRepository>;

async fn find_owned(repo: &Repo, user: UserId, id: AddressId) -> Result<ShippingAddress> {
    repo.find_for_user(user, id)
        .await?
        .ok_or(DomainError::NotFound("shipping address"))
}

pub struct ListAddresses {
    repo: Repo,
}

impl ListAddresses {
    pub fn new(repo: Repo) -> Self {
        Self { repo }
    }

    pub async fn execute(&self, user: UserId) -> Result<Vec<ShippingAddress>> {
        self.repo.list_by_user(user).await
    }
}

pub struct AddAddress {
    repo: Repo,
}

impl AddAddress {
    pub fn new(repo: Repo) -> Self {
        Self { repo }
    }

    /// First address is always the default; `make_default` switches it for later ones.
    pub async fn execute(
        &self,
        user: UserId,
        details: AddressDetails,
        make_default: bool,
    ) -> Result<ShippingAddress> {
        // ponytail: count-then-insert, two concurrent requests can end at 11.
        // Lock the user row (SELECT ... FOR UPDATE) in the repo if that ever matters.
        let count = self.repo.count_by_user(user).await?;
        if count >= MAX_ADDRESSES_PER_USER {
            return Err(DomainError::Conflict(format!(
                "maximum {MAX_ADDRESSES_PER_USER} shipping addresses per user"
            )));
        }
        let mut address = ShippingAddress::new(user, details, count == 0)?;
        self.repo.insert(&address).await?;
        if make_default && !address.is_default {
            self.repo.set_default(user, address.id).await?;
            address.is_default = true;
        }
        Ok(address)
    }
}

pub struct UpdateAddress {
    repo: Repo,
}

impl UpdateAddress {
    pub fn new(repo: Repo) -> Self {
        Self { repo }
    }

    /// Full replace of the details (PUT semantics).
    pub async fn execute(
        &self,
        user: UserId,
        id: AddressId,
        details: AddressDetails,
    ) -> Result<ShippingAddress> {
        let mut address = find_owned(&self.repo, user, id).await?;
        address.update_details(details)?;
        self.repo.update(&address).await?;
        Ok(address)
    }
}

pub struct DeleteAddress {
    repo: Repo,
}

impl DeleteAddress {
    pub fn new(repo: Repo) -> Self {
        Self { repo }
    }

    /// Deleting the default promotes the newest remaining address.
    pub async fn execute(&self, user: UserId, id: AddressId) -> Result<()> {
        let address = find_owned(&self.repo, user, id).await?;
        self.repo.delete(user, id).await?;
        if address.is_default
            && let Some(next) = self.repo.list_by_user(user).await?.first()
        {
            self.repo.set_default(user, next.id).await?;
        }
        Ok(())
    }
}

pub struct SetDefaultAddress {
    repo: Repo,
}

impl SetDefaultAddress {
    pub fn new(repo: Repo) -> Self {
        Self { repo }
    }

    pub async fn execute(&self, user: UserId, id: AddressId) -> Result<ShippingAddress> {
        let mut address = find_owned(&self.repo, user, id).await?;
        if !address.is_default {
            self.repo.set_default(user, id).await?;
            address.is_default = true;
        }
        Ok(address)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::FakeAddresses;

    fn details(label: &str) -> AddressDetails {
        AddressDetails {
            label: label.into(),
            recipient_name: "Alfian".into(),
            phone: "+62 812-3456-7890".into(),
            address_line: "Jl. Pajajaran No. 1".into(),
            city: "Bogor".into(),
            province: "Jawa Barat".into(),
            postal_code: "16143".into(),
            notes: Some("  ".into()),
        }
    }

    fn defaults(list: &[ShippingAddress]) -> Vec<&str> {
        list.iter()
            .filter(|a| a.is_default)
            .map(|a| a.details.label.as_str())
            .collect()
    }

    #[tokio::test]
    async fn first_is_default_and_make_default_switches() {
        let repo: Repo = Arc::new(FakeAddresses::default());
        let (add, list) = (AddAddress::new(repo.clone()), ListAddresses::new(repo));
        let user = UserId::generate();

        let home = add.execute(user, details("Rumah"), false).await.unwrap();
        assert!(home.is_default);
        assert_eq!(home.details.phone, "+6281234567890");
        assert_eq!(home.details.notes, None);

        let office = add.execute(user, details("Kantor"), false).await.unwrap();
        assert!(!office.is_default);

        add.execute(user, details("Kos"), true).await.unwrap();
        let all = list.execute(user).await.unwrap();
        assert_eq!(defaults(&all), ["Kos"]);
        assert_eq!(all[0].details.label, "Kos", "default listed first");
    }

    #[tokio::test]
    async fn max_ten_per_user() {
        let repo: Repo = Arc::new(FakeAddresses::default());
        let add = AddAddress::new(repo);
        let user = UserId::generate();
        for i in 0..MAX_ADDRESSES_PER_USER {
            add.execute(user, details(&format!("A{i}")), false)
                .await
                .unwrap();
        }
        assert!(matches!(
            add.execute(user, details("extra"), false).await,
            Err(DomainError::Conflict(_))
        ));
        // Limit is per user.
        assert!(
            add.execute(UserId::generate(), details("other"), false)
                .await
                .is_ok()
        );
    }

    #[tokio::test]
    async fn deleting_default_promotes_newest() {
        let repo: Repo = Arc::new(FakeAddresses::default());
        let add = AddAddress::new(repo.clone());
        let user = UserId::generate();
        let first = add.execute(user, details("A"), false).await.unwrap();
        add.execute(user, details("B"), false).await.unwrap();
        add.execute(user, details("C"), false).await.unwrap();

        DeleteAddress::new(repo.clone())
            .execute(user, first.id)
            .await
            .unwrap();
        let all = ListAddresses::new(repo).execute(user).await.unwrap();
        assert_eq!(all.len(), 2);
        assert_eq!(defaults(&all), ["C"]);
    }

    #[tokio::test]
    async fn cannot_touch_another_users_address() {
        let repo: Repo = Arc::new(FakeAddresses::default());
        let (owner, intruder) = (UserId::generate(), UserId::generate());
        let addr = AddAddress::new(repo.clone())
            .execute(owner, details("Rumah"), false)
            .await
            .unwrap();
        let not_found = Some(DomainError::NotFound("shipping address"));

        let upd = UpdateAddress::new(repo.clone());
        assert_eq!(
            upd.execute(intruder, addr.id, details("x")).await.err(),
            not_found
        );
        let del = DeleteAddress::new(repo.clone());
        assert_eq!(del.execute(intruder, addr.id).await.err(), not_found);
        let set = SetDefaultAddress::new(repo.clone());
        assert_eq!(set.execute(intruder, addr.id).await.err(), not_found);
        assert_eq!(
            ListAddresses::new(repo).execute(owner).await.unwrap().len(),
            1
        );
    }

    #[tokio::test]
    async fn validation() {
        let repo: Repo = Arc::new(FakeAddresses::default());
        let add = AddAddress::new(repo);
        let user = UserId::generate();
        let cases = [
            AddressDetails {
                postal_code: "1614".into(),
                ..details("A")
            },
            AddressDetails {
                postal_code: "1614a".into(),
                ..details("A")
            },
            AddressDetails {
                phone: "12345".into(),
                ..details("A")
            },
            AddressDetails {
                phone: "0812abc45678".into(),
                ..details("A")
            },
            AddressDetails {
                city: "  ".into(),
                ..details("A")
            },
            details(&"x".repeat(31)),
        ];
        for d in cases {
            assert!(
                matches!(
                    add.execute(user, d.clone(), false).await,
                    Err(DomainError::Validation(_))
                ),
                "{d:?} should fail"
            );
        }
    }
}
