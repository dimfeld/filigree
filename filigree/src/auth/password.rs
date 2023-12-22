use argon2::{
    password_hash::{PasswordHash, PasswordHasher, PasswordVerifier, SaltString},
    Argon2,
};
use uuid::Uuid;

use super::AuthError;

/// Hash a password using a randomly-generated salt value
pub fn new_hash(password: &str) -> Result<String, AuthError> {
    let salt = uuid::Uuid::new_v4();
    hash_password(password, &salt)
}

fn hash_password(password: &str, salt: &Uuid) -> Result<String, AuthError> {
    let saltstring = SaltString::encode_b64(salt.as_bytes())
        .map_err(|e| AuthError::PasswordHasherError(e.to_string()))?;

    let hash = Argon2::default()
        .hash_password(password.as_bytes(), saltstring.as_salt())
        .map_err(|e| AuthError::PasswordHasherError(e.to_string()))?;

    Ok(hash.to_string())
}

/// Verify that the given password matches the stored hash
pub fn verify_password(password: &str, hash_str: &str) -> Result<(), AuthError> {
    let hash =
        PasswordHash::new(hash_str).map_err(|e| AuthError::PasswordHasherError(e.to_string()))?;

    Argon2::default()
        .verify_password(password.as_bytes(), &hash)
        .map_err(|_| AuthError::Unauthenticated)
}

#[cfg(all(test, any(test_slow, test_password)))]
mod tests {
    use super::*;
    use crate::error::Result;

    #[test]
    fn good_password() -> Result<()> {
        let hash = new_hash("abcdef")?;
        verify_password("abcdef", &hash)
    }

    #[test]
    fn bad_password() -> Result<()> {
        let hash = new_hash("abcdef")?;
        verify_password("abcdefg", &hash).expect_err("non-matching password");
        Ok(())
    }

    #[test]
    fn unique_password_salt() {
        let p1 = new_hash("abc").unwrap();
        let p2 = new_hash("abc").unwrap();
        assert_ne!(p1, p2);
    }
}
