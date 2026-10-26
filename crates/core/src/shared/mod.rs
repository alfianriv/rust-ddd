mod error;
pub mod ports;

pub use error::{DomainError, Result};

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
