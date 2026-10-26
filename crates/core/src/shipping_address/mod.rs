mod application;
mod domain;

pub use application::{AddAddress, DeleteAddress, ListAddresses, SetDefaultAddress, UpdateAddress};
pub use domain::{
    AddressDetails, AddressId, MAX_ADDRESSES_PER_USER, ShippingAddress, ShippingAddressRepository,
};
