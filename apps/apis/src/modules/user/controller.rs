use axum::extract::State;
use axum::http::StatusCode;

use super::dto::{LoginDto, RegisterDto, UpdateProfileDto};
use super::response::{LoginResponse, UserResponse};
use crate::AppState;
use crate::error::{ApiError, ErrorBody};
use crate::extract::{AuthUser, Json};

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
        (status = OK, body = LoginResponse),
        (status = UNAUTHORIZED, description = "Wrong email or password", body = ErrorBody),
    ))]
pub async fn login(
    State(c): State<AppState>,
    Json(dto): Json<LoginDto>,
) -> Result<Json<LoginResponse>, ApiError> {
    let out = c.login.execute(&dto.email, &dto.password).await?;
    Ok(Json(out.into()))
}

#[utoipa::path(get, path = "/me", tag = "me", security(("bearer" = [])),
    responses(
        (status = OK, body = UserResponse),
        (status = UNAUTHORIZED, body = ErrorBody),
    ))]
pub async fn get_me(
    State(c): State<AppState>,
    AuthUser(me): AuthUser,
) -> Result<Json<UserResponse>, ApiError> {
    let user = c.get_user.execute(me).await?;
    Ok(Json(user.into()))
}

#[utoipa::path(patch, path = "/me", tag = "me", security(("bearer" = [])),
    request_body = UpdateProfileDto,
    responses(
        (status = OK, body = UserResponse),
        (status = UNAUTHORIZED, body = ErrorBody),
        (status = UNPROCESSABLE_ENTITY, body = ErrorBody),
    ))]
pub async fn update_me(
    State(c): State<AppState>,
    AuthUser(me): AuthUser,
    Json(dto): Json<UpdateProfileDto>,
) -> Result<Json<UserResponse>, ApiError> {
    let user = c.update_profile.execute(me, &dto.name).await?;
    Ok(Json(user.into()))
}
