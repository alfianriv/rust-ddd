use app_core::shipping_address::AddressId;
use axum::extract::State;
use axum::http::StatusCode;
use uuid::Uuid;

use super::dto::{AddressDto, CreateAddressDto};
use super::response::AddressResponse;
use crate::AppState;
use crate::error::{ApiError, ErrorBody};
use crate::extract::{AuthUser, Json, Path};

#[utoipa::path(get, path = "/me/addresses", tag = "shipping addresses", security(("bearer" = [])),
    responses(
        (status = OK, description = "Default first, then newest first", body = Vec<AddressResponse>),
        (status = UNAUTHORIZED, body = ErrorBody),
    ))]
pub async fn list_addresses(
    State(c): State<AppState>,
    AuthUser(me): AuthUser,
) -> Result<Json<Vec<AddressResponse>>, ApiError> {
    let list = c.list_addresses.execute(me).await?;
    Ok(Json(list.into_iter().map(Into::into).collect()))
}

#[utoipa::path(post, path = "/me/addresses", tag = "shipping addresses", security(("bearer" = [])),
    request_body = CreateAddressDto,
    responses(
        (status = CREATED, body = AddressResponse),
        (status = CONFLICT, description = "Already at 10 addresses", body = ErrorBody),
        (status = UNPROCESSABLE_ENTITY, body = ErrorBody),
    ))]
pub async fn add_address(
    State(c): State<AppState>,
    AuthUser(me): AuthUser,
    Json(dto): Json<CreateAddressDto>,
) -> Result<(StatusCode, Json<AddressResponse>), ApiError> {
    let address = c
        .add_address
        .execute(me, dto.address.into(), dto.is_default)
        .await?;
    Ok((StatusCode::CREATED, Json(address.into())))
}

#[utoipa::path(put, path = "/me/addresses/{id}", tag = "shipping addresses", security(("bearer" = [])),
    params(("id" = Uuid, Path)),
    request_body = AddressDto,
    responses(
        (status = OK, body = AddressResponse),
        (status = NOT_FOUND, body = ErrorBody),
        (status = UNPROCESSABLE_ENTITY, body = ErrorBody),
    ))]
pub async fn update_address(
    State(c): State<AppState>,
    AuthUser(me): AuthUser,
    Path(id): Path<Uuid>,
    Json(dto): Json<AddressDto>,
) -> Result<Json<AddressResponse>, ApiError> {
    let address = c
        .update_address
        .execute(me, AddressId(id), dto.into())
        .await?;
    Ok(Json(address.into()))
}

#[utoipa::path(delete, path = "/me/addresses/{id}", tag = "shipping addresses", security(("bearer" = [])),
    params(("id" = Uuid, Path)),
    responses(
        (status = NO_CONTENT, description = "Deleted. If it was the default, the newest remaining one becomes default."),
        (status = NOT_FOUND, body = ErrorBody),
    ))]
pub async fn delete_address(
    State(c): State<AppState>,
    AuthUser(me): AuthUser,
    Path(id): Path<Uuid>,
) -> Result<StatusCode, ApiError> {
    c.delete_address.execute(me, AddressId(id)).await?;
    Ok(StatusCode::NO_CONTENT)
}

#[utoipa::path(post, path = "/me/addresses/{id}/default", tag = "shipping addresses", security(("bearer" = [])),
    params(("id" = Uuid, Path)),
    responses(
        (status = OK, body = AddressResponse),
        (status = NOT_FOUND, body = ErrorBody),
    ))]
pub async fn set_default_address(
    State(c): State<AppState>,
    AuthUser(me): AuthUser,
    Path(id): Path<Uuid>,
) -> Result<Json<AddressResponse>, ApiError> {
    let address = c.set_default_address.execute(me, AddressId(id)).await?;
    Ok(Json(address.into()))
}
