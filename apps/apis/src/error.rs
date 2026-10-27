use app_core::shared::DomainError;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use serde::Serialize;
use utoipa::ToSchema;

/// Every error leaves the API as `{"error": {"code", "message"}}`.
#[derive(Debug)]
pub enum ApiError {
    Domain(DomainError),
    /// Malformed request (bad JSON, bad path param). Message comes from axum.
    BadRequest(String),
}

impl From<DomainError> for ApiError {
    fn from(e: DomainError) -> Self {
        Self::Domain(e)
    }
}

#[derive(Serialize, ToSchema)]
pub struct ErrorBody {
    error: ErrorDetail,
}

#[derive(Serialize, ToSchema)]
pub struct ErrorDetail {
    /// validation_error | not_found | conflict | unauthorized | bad_request | internal_error
    code: &'static str,
    message: String,
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let (status, code, message) = match self {
            Self::BadRequest(m) => (StatusCode::BAD_REQUEST, "bad_request", m),
            Self::Domain(e) => match e {
                DomainError::Validation(m) => {
                    (StatusCode::UNPROCESSABLE_ENTITY, "validation_error", m)
                }
                DomainError::NotFound(_) => (StatusCode::NOT_FOUND, "not_found", e.to_string()),
                DomainError::Conflict(m) => (StatusCode::CONFLICT, "conflict", m),
                DomainError::Unauthorized => {
                    (StatusCode::UNAUTHORIZED, "unauthorized", e.to_string())
                }
                DomainError::Infra(detail) => {
                    // Details go to logs only; clients get a generic message.
                    tracing::error!(error = %detail, "infrastructure error");
                    (
                        StatusCode::INTERNAL_SERVER_ERROR,
                        "internal_error",
                        "internal server error".into(),
                    )
                }
            },
        };
        let body = ErrorBody {
            error: ErrorDetail { code, message },
        };
        (status, axum::Json(body)).into_response()
    }
}
