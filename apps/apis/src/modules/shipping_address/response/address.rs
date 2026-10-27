use app_core::shipping_address::ShippingAddress;
use chrono::{DateTime, Utc};
use serde::Serialize;
use utoipa::ToSchema;
use uuid::Uuid;

#[derive(Serialize, ToSchema)]
pub struct AddressResponse {
    pub id: Uuid,
    pub label: String,
    pub recipient_name: String,
    pub phone: String,
    pub address_line: String,
    pub city: String,
    pub province: String,
    pub postal_code: String,
    pub notes: Option<String>,
    pub is_default: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl From<ShippingAddress> for AddressResponse {
    fn from(a: ShippingAddress) -> Self {
        let d = a.details;
        Self {
            id: a.id.0,
            label: d.label,
            recipient_name: d.recipient_name,
            phone: d.phone,
            address_line: d.address_line,
            city: d.city,
            province: d.province,
            postal_code: d.postal_code,
            notes: d.notes,
            is_default: a.is_default,
            created_at: a.created_at,
            updated_at: a.updated_at,
        }
    }
}
