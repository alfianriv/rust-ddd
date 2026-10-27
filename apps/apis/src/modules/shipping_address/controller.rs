//! Owner = the user in the JWT. Use cases enforce ownership: someone else's address -> 403.
use app_core::shipping_address::AddressId;
use axum::extract::State;
use axum::http::StatusCode;
use uuid::Uuid;

use super::dto::{AddressDto, CreateAddressDto};
use super::response::AddressResponse;
use crate::AppState;
use crate::error::{ApiError, ErrorBody};
use crate::extract::{AuthUser, Json, Path};

#[utoipa::path(get, path = "/addresses", tag = "addresses", security(("bearer" = [])),
    responses(
        (status = OK, description = "The caller's addresses. Default first, then newest first", body = Vec<AddressResponse>),
        (status = UNAUTHORIZED, body = ErrorBody),
    ))]
pub async fn list_addresses(
    State(c): State<AppState>,
    AuthUser(user): AuthUser,
) -> Result<Json<Vec<AddressResponse>>, ApiError> {
    let list = c.list_addresses.execute(user).await?;
    Ok(Json(list.into_iter().map(Into::into).collect()))
}

#[utoipa::path(post, path = "/addresses", tag = "addresses", security(("bearer" = [])),
    request_body = CreateAddressDto,
    responses(
        (status = CREATED, description = "Owned by the caller", body = AddressResponse),
        (status = UNAUTHORIZED, body = ErrorBody),
        (status = CONFLICT, description = "Already at 10 addresses", body = ErrorBody),
        (status = UNPROCESSABLE_ENTITY, body = ErrorBody),
    ))]
pub async fn add_address(
    State(c): State<AppState>,
    AuthUser(user): AuthUser,
    Json(dto): Json<CreateAddressDto>,
) -> Result<(StatusCode, Json<AddressResponse>), ApiError> {
    let address = c
        .add_address
        .execute(user, dto.address.into(), dto.is_default)
        .await?;
    Ok((StatusCode::CREATED, Json(address.into())))
}

#[utoipa::path(put, path = "/addresses/{id}", tag = "addresses", security(("bearer" = [])),
    params(("id" = Uuid, Path)),
    request_body = AddressDto,
    responses(
        (status = OK, body = AddressResponse),
        (status = UNAUTHORIZED, body = ErrorBody),
        (status = FORBIDDEN, description = "Address belongs to another user", body = ErrorBody),
        (status = NOT_FOUND, body = ErrorBody),
        (status = UNPROCESSABLE_ENTITY, body = ErrorBody),
    ))]
pub async fn update_address(
    State(c): State<AppState>,
    AuthUser(user): AuthUser,
    Path(id): Path<Uuid>,
    Json(dto): Json<AddressDto>,
) -> Result<Json<AddressResponse>, ApiError> {
    let address = c
        .update_address
        .execute(user, AddressId(id), dto.into())
        .await?;
    Ok(Json(address.into()))
}

#[utoipa::path(delete, path = "/addresses/{id}", tag = "addresses", security(("bearer" = [])),
    params(("id" = Uuid, Path)),
    responses(
        (status = NO_CONTENT, description = "Deleted. If it was the default, the newest remaining one becomes default."),
        (status = UNAUTHORIZED, body = ErrorBody),
        (status = FORBIDDEN, description = "Address belongs to another user", body = ErrorBody),
        (status = NOT_FOUND, body = ErrorBody),
    ))]
pub async fn delete_address(
    State(c): State<AppState>,
    AuthUser(user): AuthUser,
    Path(id): Path<Uuid>,
) -> Result<StatusCode, ApiError> {
    c.delete_address.execute(user, AddressId(id)).await?;
    Ok(StatusCode::NO_CONTENT)
}

#[utoipa::path(put, path = "/addresses/{id}/default", tag = "addresses", security(("bearer" = [])),
    params(("id" = Uuid, Path)),
    responses(
        (status = OK, description = "Idempotent: already-default is a no-op", body = AddressResponse),
        (status = UNAUTHORIZED, body = ErrorBody),
        (status = FORBIDDEN, description = "Address belongs to another user", body = ErrorBody),
        (status = NOT_FOUND, body = ErrorBody),
    ))]
pub async fn set_default_address(
    State(c): State<AppState>,
    AuthUser(user): AuthUser,
    Path(id): Path<Uuid>,
) -> Result<Json<AddressResponse>, ApiError> {
    let address = c.set_default_address.execute(user, AddressId(id)).await?;
    Ok(Json(address.into()))
}
