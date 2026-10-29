# 0003 — Owner password and session tokens

- **Status**: accepted
- **Context**: Spec §9 needs one owner with an argon2 password, and sessions usable both as a CLI bearer token and a web cookie (task 004).
- **Decision**: The password is hashed with argon2id (`argon2` crate defaults) as a PHC string in `owner` (one row, `CHECK (id = 1)`). A session token is 32 random bytes from the OS (`getrandom`), hex-encoded; the same token is returned as JSON and set as the `polykov_session` cookie (HttpOnly, SameSite=Strict, Path=/). The DB keeps only its SHA-256 hex plus `expires_at` (Unix seconds, 30 days).
- **Consequences**: A leaked DB does not leak usable sessions, and a fast hash is enough because tokens are high-entropy. Never store plain tokens or switch the lookup to a slow hash. Changing the TTL only affects new sessions.
