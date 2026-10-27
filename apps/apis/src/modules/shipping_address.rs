use app_core::shipping_address::{AddressDetails, AddressId, ShippingAddress};
use axum::extract::State;
use axum::http::StatusCode;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use utoipa_axum::router::OpenApiRouter;
use utoipa_axum::routes;
use uuid::Uuid;

use crate::AppState;
use crate::error::{ApiError, ErrorBody};
use crate::extract::{AuthUser, Json, Path};

pub fn routes() -> OpenApiRouter<AppState> {
    OpenApiRouter::new()
        .routes(routes!(list_addresses, add_address))
        .routes(routes!(update_address, delete_address))
        .routes(routes!(set_default_address))
}

#[derive(Deserialize, ToSchema)]
pub struct AddressRequest {
    /// e.g. "Rumah", "Kantor". 1-30 characters.
    label: String,
    recipient_name: String,
    /// 8-15 digits, optional leading `+`. Spaces and dashes are stripped.
    phone: String,
    address_line: String,
    city: String,
    province: String,
    /// 5 digits.
    postal_code: String,
    notes: Option<String>,
}

impl From<AddressRequest> for AddressDetails {
    fn from(r: AddressRequest) -> Self {
        Self {
            label: r.label,
            recipient_name: r.recipient_name,
            phone: r.phone,
            address_line: r.address_line,
            city: r.city,
            province: r.province,
            postal_code: r.postal_code,
            notes: r.notes,
        }
    }
}

#[derive(Deserialize, ToSchema)]
pub struct CreateAddressRequest {
    #[serde(flatten)]
    address: AddressRequest,
    /// Make this the default. The first address is always the default.
    #[serde(default)]
    is_default: bool,
}

#[derive(Serialize, ToSchema)]
pub struct AddressResponse {
    id: Uuid,
    label: String,
    recipient_name: String,
    phone: String,
    address_line: String,
    city: String,
    province: String,
    postal_code: String,
    notes: Option<String>,
    is_default: bool,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
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

#[utoipa::path(get, path = "/me/addresses", tag = "shipping addresses", security(("bearer" = [])),
    responses(
        (status = OK, description = "Default first, then newest first", body = Vec<AddressResponse>),
        (status = UNAUTHORIZED, body = ErrorBody),
    ))]
async fn list_addresses(
    State(c): State<AppState>,
    AuthUser(me): AuthUser,
) -> Result<Json<Vec<AddressResponse>>, ApiError> {
    let list = c.list_addresses.execute(me).await?;
    Ok(Json(list.into_iter().map(Into::into).collect()))
}

#[utoipa::path(post, path = "/me/addresses", tag = "shipping addresses", security(("bearer" = [])),
    request_body = CreateAddressRequest,
    responses(
        (status = CREATED, body = AddressResponse),
        (status = CONFLICT, description = "Already at 10 addresses", body = ErrorBody),
        (status = UNPROCESSABLE_ENTITY, body = ErrorBody),
    ))]
async fn add_address(
    State(c): State<AppState>,
    AuthUser(me): AuthUser,
    Json(req): Json<CreateAddressRequest>,
) -> Result<(StatusCode, Json<AddressResponse>), ApiError> {
    let address = c
        .add_address
        .execute(me, req.address.into(), req.is_default)
        .await?;
    Ok((StatusCode::CREATED, Json(address.into())))
}

#[utoipa::path(put, path = "/me/addresses/{id}", tag = "shipping addresses", security(("bearer" = [])),
    params(("id" = Uuid, Path)),
    request_body = AddressRequest,
    responses(
        (status = OK, body = AddressResponse),
        (status = NOT_FOUND, body = ErrorBody),
        (status = UNPROCESSABLE_ENTITY, body = ErrorBody),
    ))]
async fn update_address(
    State(c): State<AppState>,
    AuthUser(me): AuthUser,
    Path(id): Path<Uuid>,
    Json(req): Json<AddressRequest>,
) -> Result<Json<AddressResponse>, ApiError> {
    let address = c
        .update_address
        .execute(me, AddressId(id), req.into())
        .await?;
    Ok(Json(address.into()))
}

#[utoipa::path(delete, path = "/me/addresses/{id}", tag = "shipping addresses", security(("bearer" = [])),
    params(("id" = Uuid, Path)),
    responses(
        (status = NO_CONTENT, description = "Deleted. If it was the default, the newest remaining one becomes default."),
        (status = NOT_FOUND, body = ErrorBody),
    ))]
async fn delete_address(
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
async fn set_default_address(
    State(c): State<AppState>,
    AuthUser(me): AuthUser,
    Path(id): Path<Uuid>,
) -> Result<Json<AddressResponse>, ApiError> {
    let address = c.set_default_address.execute(me, AddressId(id)).await?;
    Ok(Json(address.into()))
}
