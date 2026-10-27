use axum::extract::State;
use axum::http::StatusCode;

use super::dto::{LoginDto, RegisterDto, UpdateUserDto};
use super::response::{LoginResponse, UserResponse};
use crate::AppState;
use crate::error::{ApiError, ErrorBody};
use crate::extract::{Json, OwnedUser};

#[utoipa::path(post, path = "/auth/register", tag = "auth", request_body = RegisterDto,
    responses(
        (status = CREATED, body = UserResponse),
        (status = CONFLICT, description = "Email already registered", body = ErrorBody),
        (status = UNPROCESSABLE_ENTITY, body = ErrorBody),
    ))]
pub async fn register(
    State(c): State<AppState>,
    Json(dto): Json<RegisterDto>,
) -> Result<(StatusCode, Json<UserResponse>), ApiError> {
    let user = c.register_user.execute(dto.into()).await?;
    Ok((StatusCode::CREATED, Json(user.into())))
}

#[utoipa::path(post, path = "/auth/login", tag = "auth", request_body = LoginDto,
    responses(
        (status = OK, description = "`user.id` is the `{id}` for `/users/{id}`", body = LoginResponse),
        (status = UNAUTHORIZED, description = "Wrong email or password", body = ErrorBody),
    ))]
pub async fn login(
    State(c): State<AppState>,
    Json(dto): Json<LoginDto>,
) -> Result<Json<LoginResponse>, ApiError> {
    let out = c.login.execute(&dto.email, &dto.password).await?;
    Ok(Json(out.into()))
}

#[utoipa::path(get, path = "/users/{id}", tag = "users", security(("bearer" = [])),
    params(("id" = uuid::Uuid, Path, description = "Must be the id in the JWT")),
    responses(
        (status = OK, body = UserResponse),
        (status = UNAUTHORIZED, body = ErrorBody),
        (status = FORBIDDEN, description = "Not your id", body = ErrorBody),
    ))]
pub async fn get_user(
    State(c): State<AppState>,
    OwnedUser(user): OwnedUser,
) -> Result<Json<UserResponse>, ApiError> {
    let user = c.get_user.execute(user).await?;
    Ok(Json(user.into()))
}

#[utoipa::path(put, path = "/users/{id}", tag = "users", security(("bearer" = [])),
    params(("id" = uuid::Uuid, Path, description = "Must be the id in the JWT")),
    request_body = UpdateUserDto,
    responses(
        (status = OK, body = UserResponse),
        (status = UNAUTHORIZED, body = ErrorBody),
        (status = FORBIDDEN, description = "Not your id", body = ErrorBody),
        (status = UNPROCESSABLE_ENTITY, body = ErrorBody),
    ))]
pub async fn update_user(
    State(c): State<AppState>,
    OwnedUser(user): OwnedUser,
    Json(dto): Json<UpdateUserDto>,
) -> Result<Json<UserResponse>, ApiError> {
    let user = c.update_profile.execute(user, &dto.name).await?;
    Ok(Json(user.into()))
}
