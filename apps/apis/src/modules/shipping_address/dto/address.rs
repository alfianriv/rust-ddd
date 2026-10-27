use app_core::shipping_address::AddressDetails;
use serde::Deserialize;
use utoipa::ToSchema;

/// Address fields. Used as-is for `PUT /me/addresses/{id}` (full replace).
#[derive(Deserialize, ToSchema)]
pub struct AddressDto {
    /// e.g. "Rumah", "Kantor". 1-30 characters.
    pub label: String,
    pub recipient_name: String,
    /// 8-15 digits, optional leading `+`. Spaces and dashes are stripped.
    pub phone: String,
    pub address_line: String,
    pub city: String,
    pub province: String,
    /// 5 digits.
    pub postal_code: String,
    pub notes: Option<String>,
}

impl From<AddressDto> for AddressDetails {
    fn from(d: AddressDto) -> Self {
        Self {
            label: d.label,
            recipient_name: d.recipient_name,
            phone: d.phone,
            address_line: d.address_line,
            city: d.city,
            province: d.province,
            postal_code: d.postal_code,
            notes: d.notes,
        }
    }
}
