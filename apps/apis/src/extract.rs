//! Extractors whose rejections use the same JSON error shape as everything else.
use app_core::shared::DomainError;
use app_core::user::UserId;
use axum::extract::rejection::{JsonRejection, PathRejection};
use axum::extract::{FromRequest, FromRequestParts, RawPathParams};
use axum::http::header::AUTHORIZATION;
use axum::http::request::Parts;
use axum::response::{IntoResponse, Response};
use serde::Serialize;

use crate::AppState;
use crate::error::ApiError;

#[derive(FromRequest)]
#[from_request(via(axum::Json), rejection(ApiError))]
pub struct Json<T>(pub T);

impl<T: Serialize> IntoResponse for Json<T> {
    fn into_response(self) -> Response {
        axum::Json(self.0).into_response()
    }
}

impl From<JsonRejection> for ApiError {
    fn from(r: JsonRejection) -> Self {
        Self::BadRequest(r.body_text())
    }
}

#[derive(FromRequestParts)]
#[from_request(via(axum::extract::Path), rejection(ApiError))]
pub struct Path<T>(pub T);

impl From<PathRejection> for ApiError {
    fn from(r: PathRejection) -> Self {
        Self::BadRequest(r.body_text())
    }
}

/// The caller, from `Authorization: Bearer <jwt>`. Missing/invalid/expired -> 401.
pub struct AuthUser(pub UserId);

impl FromRequestParts<AppState> for AuthUser {
    type Rejection = ApiError;

    async fn from_request_parts(parts: &mut Parts, state: &AppState) -> Result<Self, ApiError> {
        let token = parts
            .headers
            .get(AUTHORIZATION)
            .and_then(|v| v.to_str().ok())
            .and_then(|v| v.strip_prefix("Bearer "))
            .ok_or(DomainError::Unauthorized)?;
        Ok(Self(state.tokens.verify(token)?))
    }
}

/// Ownership guard for every route under `/users/{user_id}`: the caller must be that user.
/// No token -> 401, `user_id` not a UUID -> 400, someone else's `user_id` -> 403.
/// Holds the verified user id; handlers use it instead of reading the path again.
pub struct OwnedUser(pub UserId);

impl FromRequestParts<AppState> for OwnedUser {
    type Rejection = ApiError;

    async fn from_request_parts(parts: &mut Parts, state: &AppState) -> Result<Self, ApiError> {
        let AuthUser(caller) = AuthUser::from_request_parts(parts, state).await?;
        let params = RawPathParams::from_request_parts(parts, state)
            .await
            .map_err(|r| ApiError::BadRequest(r.body_text()))?;
        let user_id: uuid::Uuid = params
            .iter()
            .find(|(k, _)| *k == "user_id")
            .and_then(|(_, v)| v.parse().ok())
            .ok_or_else(|| ApiError::BadRequest("invalid user_id".into()))?;
        if user_id != caller.0 {
            return Err(ApiError::Forbidden);
        }
        Ok(Self(caller))
    }
}
