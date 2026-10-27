use app_core::user::LoginOutput;
use serde::Serialize;
use utoipa::ToSchema;

use super::UserResponse;

#[derive(Serialize, ToSchema)]
pub struct LoginResponse {
    pub access_token: String,
    /// Always `Bearer`.
    pub token_type: &'static str,
    pub user: UserResponse,
}

impl From<LoginOutput> for LoginResponse {
    fn from(o: LoginOutput) -> Self {
        Self {
            access_token: o.access_token,
            token_type: "Bearer",
            user: o.user.into(),
        }
    }
}
