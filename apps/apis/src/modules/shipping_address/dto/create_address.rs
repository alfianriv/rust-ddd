use serde::Deserialize;
use utoipa::ToSchema;

use super::AddressDto;

/// Address fields + `is_default`, flat in the JSON body.
#[derive(Deserialize, ToSchema)]
pub struct CreateAddressDto {
    #[serde(flatten)]
    pub address: AddressDto,
    /// Make this the default. The first address is always the default.
    #[serde(default)]
    pub is_default: bool,
}
