use serde::Deserialize;
use utoipa::ToSchema;

#[derive(Deserialize, ToSchema)]
pub struct UpdateProfileDto {
    /// 1-100 characters.
    pub name: String,
}
