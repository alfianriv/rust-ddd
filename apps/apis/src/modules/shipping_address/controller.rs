use app_core::shipping_address::AddressId;
use axum::extract::State;
use axum::http::StatusCode;
use serde::Deserialize;
use uuid::Uuid;

use super::dto::{AddressDto, CreateAddressDto};
use super::response::AddressResponse;
use crate::AppState;
use crate::error::{ApiError, ErrorBody};
use crate::extract::{Json, OwnedUser, Path};

/// `{id}` of `/users/{user_id}/addresses/{id}`. `user_id` is checked by `OwnedUser`;
/// the address is looked up scoped to that user, so someone else's address -> 404.
#[derive(Deserialize)]
pub struct AddressPath {
    id: Uuid,
}

#[utoipa::path(get, path = "/users/{user_id}/addresses", tag = "shipping addresses", security(("bearer" = [])),
    params(("user_id" = Uuid, Path, description = "Must be the caller's own id")),
    responses(
        (status = OK, description = "Default first, then newest first", body = Vec<AddressResponse>),
        (status = UNAUTHORIZED, body = ErrorBody),
        (status = FORBIDDEN, description = "Not your user_id", body = ErrorBody),
    ))]
pub async fn list_addresses(
    State(c): State<AppState>,
    OwnedUser(user): OwnedUser,
) -> Result<Json<Vec<AddressResponse>>, ApiError> {
    let list = c.list_addresses.execute(user).await?;
    Ok(Json(list.into_iter().map(Into::into).collect()))
}

#[utoipa::path(post, path = "/users/{user_id}/addresses", tag = "shipping addresses", security(("bearer" = [])),
    params(("user_id" = Uuid, Path, description = "Must be the caller's own id")),
    request_body = CreateAddressDto,
    responses(
        (status = CREATED, body = AddressResponse),
        (status = FORBIDDEN, description = "Not your user_id", body = ErrorBody),
        (status = CONFLICT, description = "Already at 10 addresses", body = ErrorBody),
        (status = UNPROCESSABLE_ENTITY, body = ErrorBody),
    ))]
pub async fn add_address(
    State(c): State<AppState>,
    OwnedUser(user): OwnedUser,
    Json(dto): Json<CreateAddressDto>,
) -> Result<(StatusCode, Json<AddressResponse>), ApiError> {
    let address = c
        .add_address
        .execute(user, dto.address.into(), dto.is_default)
        .await?;
    Ok((StatusCode::CREATED, Json(address.into())))
}

#[utoipa::path(put, path = "/users/{user_id}/addresses/{id}", tag = "shipping addresses", security(("bearer" = [])),
    params(
        ("user_id" = Uuid, Path, description = "Must be the caller's own id"),
        ("id" = Uuid, Path),
    ),
    request_body = AddressDto,
    responses(
        (status = OK, body = AddressResponse),
        (status = FORBIDDEN, description = "Not your user_id", body = ErrorBody),
        (status = NOT_FOUND, body = ErrorBody),
        (status = UNPROCESSABLE_ENTITY, body = ErrorBody),
    ))]
pub async fn update_address(
    State(c): State<AppState>,
    OwnedUser(user): OwnedUser,
    Path(path): Path<AddressPath>,
    Json(dto): Json<AddressDto>,
) -> Result<Json<AddressResponse>, ApiError> {
    let address = c
        .update_address
        .execute(user, AddressId(path.id), dto.into())
        .await?;
    Ok(Json(address.into()))
}

#[utoipa::path(delete, path = "/users/{user_id}/addresses/{id}", tag = "shipping addresses", security(("bearer" = [])),
    params(
        ("user_id" = Uuid, Path, description = "Must be the caller's own id"),
        ("id" = Uuid, Path),
    ),
    responses(
        (status = NO_CONTENT, description = "Deleted. If it was the default, the newest remaining one becomes default."),
        (status = FORBIDDEN, description = "Not your user_id", body = ErrorBody),
        (status = NOT_FOUND, body = ErrorBody),
    ))]
pub async fn delete_address(
    State(c): State<AppState>,
    OwnedUser(user): OwnedUser,
    Path(path): Path<AddressPath>,
) -> Result<StatusCode, ApiError> {
    c.delete_address.execute(user, AddressId(path.id)).await?;
    Ok(StatusCode::NO_CONTENT)
}

#[utoipa::path(put, path = "/users/{user_id}/addresses/{id}/default", tag = "shipping addresses", security(("bearer" = [])),
    params(
        ("user_id" = Uuid, Path, description = "Must be the caller's own id"),
        ("id" = Uuid, Path),
    ),
    responses(
        (status = OK, description = "Idempotent: already-default is a no-op", body = AddressResponse),
        (status = FORBIDDEN, description = "Not your user_id", body = ErrorBody),
        (status = NOT_FOUND, body = ErrorBody),
    ))]
pub async fn set_default_address(
    State(c): State<AppState>,
    OwnedUser(user): OwnedUser,
    Path(path): Path<AddressPath>,
) -> Result<Json<AddressResponse>, ApiError> {
    let address = c
        .set_default_address
        .execute(user, AddressId(path.id))
        .await?;
    Ok(Json(address.into()))
}
