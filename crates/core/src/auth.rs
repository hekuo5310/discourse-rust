use argon2::{
    password_hash::{PasswordHash, PasswordHasher, PasswordVerifier, SaltString},
    Argon2,
};
use sha2::{Digest, Sha256};
use uuid::Uuid;

use crate::DomainError;

pub fn new_id() -> String {
    Uuid::new_v4().to_string()
}

pub fn new_session_token() -> String {
    format!("{}{}", Uuid::new_v4().simple(), Uuid::new_v4().simple())
}

pub fn token_digest(token: &str) -> String {
    let digest = Sha256::digest(token.as_bytes());
    digest.iter().map(|byte| format!("{byte:02x}")).collect()
}

pub fn hash_password(password: &str) -> Result<String, DomainError> {
    if password.len() < 12 || password.len() > 256 {
        return Err(DomainError::InvalidPassword);
    }
    let random = Uuid::new_v4();
    let salt = SaltString::encode_b64(random.as_bytes()).map_err(|_| DomainError::Crypto)?;
    Argon2::default()
        .hash_password(password.as_bytes(), &salt)
        .map(|hash| hash.to_string())
        .map_err(|_| DomainError::Crypto)
}

pub fn verify_password(password: &str, encoded: &str) -> bool {
    PasswordHash::new(encoded).ok().is_some_and(|hash| {
        Argon2::default()
            .verify_password(password.as_bytes(), &hash)
            .is_ok()
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn password_round_trip_and_rejects_wrong_value() {
        let encoded = hash_password("correct horse battery staple").unwrap();
        assert!(verify_password("correct horse battery staple", &encoded));
        assert!(!verify_password("incorrect horse battery staple", &encoded));
        assert!(!encoded.contains("correct horse battery staple"));
    }

    #[test]
    fn tokens_are_not_stored_verbatim() {
        let token = new_session_token();
        assert_eq!(token.len(), 64);
        assert_ne!(token, token_digest(&token));
    }
}
