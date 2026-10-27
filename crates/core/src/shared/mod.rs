mod error;
pub mod ports;

pub use error::{DomainError, Result};

/// Current time at microsecond precision, the most the DB stores. Keeps a freshly
/// created entity identical to the one read back later.
pub fn now() -> chrono::DateTime<chrono::Utc> {
    use chrono::SubsecRound;
    chrono::Utc::now().trunc_subsecs(6)
}

/// Trims and checks length in characters. Shared by every module's validation.
pub(crate) fn required_text(field: &str, value: &str, max: usize) -> Result<String> {
    let v = value.trim();
    if v.is_empty() || v.chars().count() > max {
        return Err(DomainError::Validation(format!(
            "{field} must be 1-{max} characters"
        )));
    }
    Ok(v.to_string())
}
