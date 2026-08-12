use thiserror::Error;

#[derive(Debug, Error, PartialEq, Eq)]
pub enum DomainError {
    #[error("invalid username")]
    InvalidUsername,
    #[error("invalid email")]
    InvalidEmail,
    #[error("invalid slug")]
    InvalidSlug,
    #[error("invalid password")]
    InvalidPassword,
    #[error("invalid title")]
    InvalidTitle,
    #[error("invalid body")]
    InvalidBody,
    #[error("cryptographic operation failed")]
    Crypto,
}

pub fn normalize_username(value: &str) -> Result<String, DomainError> {
    let value = value.trim().to_ascii_lowercase();
    let valid = (3..=32).contains(&value.len())
        && value.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'_' || b == b'-')
        && value.bytes().next().is_some_and(|b| b.is_ascii_alphanumeric());
    valid.then_some(value).ok_or(DomainError::InvalidUsername)
}

pub fn normalize_email(value: &str) -> Result<String, DomainError> {
    let value = value.trim().to_ascii_lowercase();
    let (local, domain) = value.split_once('@').ok_or(DomainError::InvalidEmail)?;
    if local.is_empty() || domain.is_empty() || !domain.contains('.') || value.len() > 254 {
        return Err(DomainError::InvalidEmail);
    }
    Ok(value)
}

pub fn normalize_slug(value: &str) -> Result<String, DomainError> {
    let value = value.trim().to_ascii_lowercase();
    let valid = (2..=80).contains(&value.len())
        && value.bytes().all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-')
        && !value.starts_with('-')
        && !value.ends_with('-')
        && !value.contains("--");
    valid.then_some(value).ok_or(DomainError::InvalidSlug)
}

pub fn validate_title(value: &str) -> Result<String, DomainError> {
    let value = value.trim().to_string();
    (3..=200).contains(&value.chars().count()).then_some(value).ok_or(DomainError::InvalidTitle)
}

pub fn validate_body(value: &str) -> Result<String, DomainError> {
    let value = value.trim().to_string();
    (1..=100_000).contains(&value.chars().count()).then_some(value).ok_or(DomainError::InvalidBody)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalization_is_deterministic() {
        assert_eq!(normalize_username("  Alice-1 ").unwrap(), "alice-1");
        assert_eq!(normalize_email(" A@Example.COM ").unwrap(), "a@example.com");
        assert!(normalize_slug("not valid").is_err());
    }
}

