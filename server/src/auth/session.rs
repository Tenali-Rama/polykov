//! Session store: random bearer tokens, kept in `sessions` only as a SHA-256 hash.

use rusqlite::OptionalExtension;
use sha2::{Digest, Sha256};

use super::Error;
use crate::db::{self, Db};

/// How long a session lasts from login, in seconds (30 days).
pub const TTL_SECS: i64 = 30 * 24 * 60 * 60;

/// Random bytes per token (256 bits).
const TOKEN_BYTES: usize = 32;

/// Starts a session expiring [`TTL_SECS`] from now and returns its token (64 hex chars).
/// Only the token's hash is stored, so it cannot be recovered later.
pub fn create(db: &Db) -> Result<String, Error> {
    let token = new_token()?;
    let conn = db.conn()?;
    conn.execute(
        "INSERT INTO sessions (token_hash, expires_at) VALUES (?1, unixepoch() + ?2)",
        rusqlite::params![hash_token(&token), TTL_SECS],
    )
    .map_err(db::Error::Sqlite)?;
    Ok(token)
}

/// Whether `token` belongs to a session that has not expired.
pub fn is_valid(db: &Db, token: &str) -> Result<bool, Error> {
    Ok(expires_at(db, token)?.is_some())
}

/// When the unexpired session for `token` expires, as RFC 3339 UTC
/// (`YYYY-MM-DDTHH:MM:SSZ`), or `None` if `token` is unknown or expired.
pub fn expires_at(db: &Db, token: &str) -> Result<Option<String>, Error> {
    let conn = db.conn()?;
    conn.query_row(
        "SELECT strftime('%Y-%m-%dT%H:%M:%SZ', expires_at, 'unixepoch') FROM sessions \
         WHERE token_hash = ?1 AND expires_at > unixepoch()",
        [hash_token(token)],
        |r| r.get(0),
    )
    .optional()
    .map_err(|e| db::Error::Sqlite(e).into())
}

/// Ends the session for `token`. Deleting an unknown token is not an error.
pub fn delete(db: &Db, token: &str) -> Result<(), Error> {
    let conn = db.conn()?;
    conn.execute(
        "DELETE FROM sessions WHERE token_hash = ?1",
        [hash_token(token)],
    )
    .map_err(db::Error::Sqlite)?;
    Ok(())
}

/// A fresh random token: [`TOKEN_BYTES`] from the OS, hex-encoded (64 chars).
pub(super) fn new_token() -> Result<String, Error> {
    let mut bytes = [0u8; TOKEN_BYTES];
    getrandom::fill(&mut bytes).map_err(Error::Random)?;
    Ok(hex(&bytes))
}

/// The value stored in `sessions.token_hash` (and `api_tokens.token_hash`) for `token`:
/// lowercase hex SHA-256.
pub fn hash_token(token: &str) -> String {
    hex(&Sha256::digest(token.as_bytes()))
}

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}
