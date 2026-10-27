use app_core::user::RegisterInput;
use serde::Deserialize;
use utoipa::ToSchema;

#[derive(Deserialize, ToSchema)]
pub struct RegisterDto {
    pub email: String,
    /// 1-100 characters.
    pub name: String,
    /// 8-128 characters.
    pub password: String,
}

impl From<RegisterDto> for RegisterInput {
    fn from(d: RegisterDto) -> Self {
        Self {
            email: d.email,
            name: d.name,
            password: d.password,
        }
    }
}
