use serde::Deserialize;
use utoipa::ToSchema;

/// Full replace of the editable user fields (PUT).
#[derive(Deserialize, ToSchema)]
pub struct UpdateUserDto {
    /// 1-100 characters.
    pub name: String,
}
