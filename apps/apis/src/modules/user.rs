use app_core::user::{RegisterInput, User};
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
use crate::extract::{AuthUser, Json};

pub fn routes() -> OpenApiRouter<AppState> {
    OpenApiRouter::new()
        .routes(routes!(register))
        .routes(routes!(login))
        .routes(routes!(get_me, update_me))
}

#[derive(Deserialize, ToSchema)]
pub struct RegisterRequest {
    email: String,
    name: String,
    /// 8-128 characters.
    password: String,
}

#[derive(Deserialize, ToSchema)]
pub struct LoginRequest {
    email: String,
    password: String,
}

#[derive(Deserialize, ToSchema)]
pub struct UpdateProfileRequest {
    name: String,
}

#[derive(Serialize, ToSchema)]
pub struct UserResponse {
    id: Uuid,
    email: String,
    name: String,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

impl From<User> for UserResponse {
    fn from(u: User) -> Self {
        Self {
            id: u.id.0,
            email: u.email.as_str().to_string(),
            name: u.name,
            created_at: u.created_at,
            updated_at: u.updated_at,
        }
    }
}

#[derive(Serialize, ToSchema)]
pub struct LoginResponse {
    access_token: String,
    /// Always `Bearer`.
    token_type: &'static str,
    user: UserResponse,
}

#[utoipa::path(post, path = "/auth/register", tag = "auth", request_body = RegisterRequest,
    responses(
        (status = CREATED, body = UserResponse),
        (status = CONFLICT, description = "Email already registered", body = ErrorBody),
        (status = UNPROCESSABLE_ENTITY, body = ErrorBody),
    ))]
async fn register(
    State(c): State<AppState>,
    Json(req): Json<RegisterRequest>,
) -> Result<(StatusCode, Json<UserResponse>), ApiError> {
    let user = c
        .register_user
        .execute(RegisterInput {
            email: req.email,
            name: req.name,
            password: req.password,
        })
        .await?;
    Ok((StatusCode::CREATED, Json(user.into())))
}

#[utoipa::path(post, path = "/auth/login", tag = "auth", request_body = LoginRequest,
    responses(
        (status = OK, body = LoginResponse),
        (status = UNAUTHORIZED, description = "Wrong email or password", body = ErrorBody),
    ))]
async fn login(
    State(c): State<AppState>,
    Json(req): Json<LoginRequest>,
) -> Result<Json<LoginResponse>, ApiError> {
    let out = c.login.execute(&req.email, &req.password).await?;
    Ok(Json(LoginResponse {
        access_token: out.access_token,
        token_type: "Bearer",
        user: out.user.into(),
    }))
}

#[utoipa::path(get, path = "/me", tag = "me", security(("bearer" = [])),
    responses(
        (status = OK, body = UserResponse),
        (status = UNAUTHORIZED, body = ErrorBody),
    ))]
async fn get_me(
    State(c): State<AppState>,
    AuthUser(me): AuthUser,
) -> Result<Json<UserResponse>, ApiError> {
    Ok(Json(c.get_user.execute(me).await?.into()))
}

#[utoipa::path(patch, path = "/me", tag = "me", security(("bearer" = [])),
    request_body = UpdateProfileRequest,
    responses(
        (status = OK, body = UserResponse),
        (status = UNAUTHORIZED, body = ErrorBody),
        (status = UNPROCESSABLE_ENTITY, body = ErrorBody),
    ))]
async fn update_me(
    State(c): State<AppState>,
    AuthUser(me): AuthUser,
    Json(req): Json<UpdateProfileRequest>,
) -> Result<Json<UserResponse>, ApiError> {
    Ok(Json(c.update_profile.execute(me, &req.name).await?.into()))
}
