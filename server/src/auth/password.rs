//! Owner password hashing: argon2id with the crate's default (OWASP) parameters.

use argon2::Argon2;
use argon2::password_hash::{self, PasswordHash, PasswordHasher, PasswordVerifier, SaltString};

use super::Error;

/// Hashes `password` with a fresh random salt into a PHC string (`$argon2id$...`).
pub fn hash(password: &str) -> Result<String, Error> {
    let mut salt = [0u8; 16];
    getrandom::fill(&mut salt).map_err(Error::Random)?;
    let salt = SaltString::encode_b64(&salt).map_err(Error::Hash)?;
    let hash = Argon2::default()
        .hash_password(password.as_bytes(), &salt)
        .map_err(Error::Hash)?;
    Ok(hash.to_string())
}

/// Whether `password` matches the PHC string `hash`. A malformed `hash` is an error.
pub fn verify(password: &str, hash: &str) -> Result<bool, Error> {
    let parsed = PasswordHash::new(hash).map_err(Error::Hash)?;
    match Argon2::default().verify_password(password.as_bytes(), &parsed) {
        Ok(()) => Ok(true),
        Err(password_hash::Error::Password) => Ok(false),
        Err(e) => Err(Error::Hash(e)),
    }
}
