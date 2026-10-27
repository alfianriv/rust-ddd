use serde::Deserialize;
use utoipa::ToSchema;

#[derive(Deserialize, ToSchema)]
pub struct LoginDto {
    pub email: String,
    pub password: String,
}
