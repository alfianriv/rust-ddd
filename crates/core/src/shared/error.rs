/// Single error type for use cases. Apps map variants to transport codes
/// (HTTP: Validation=422, NotFound=404, Conflict=409, Unauthorized=401, Forbidden=403, Infra=500).
#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum DomainError {
    #[error("{0}")]
    Validation(String),
    #[error("{0} not found")]
    NotFound(&'static str),
    #[error("{0}")]
    Conflict(String),
    #[error("unauthorized")]
    Unauthorized,
    /// The resource exists but belongs to someone else.
    #[error("forbidden")]
    Forbidden,
    /// Adapter failure (DB down, etc.). Message is for logs, not for clients.
    #[error("infrastructure error: {0}")]
    Infra(String),
}

pub type Result<T> = std::result::Result<T, DomainError>;
