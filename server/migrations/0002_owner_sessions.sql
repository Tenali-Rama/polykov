-- The single owner account (spec §9). The CHECK allows at most one row.
CREATE TABLE owner (
    id            INTEGER PRIMARY KEY CHECK (id = 1),
    password_hash TEXT NOT NULL,
    created_at    TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now'))
) STRICT;

-- Login sessions. Only the SHA-256 hex of each token is stored; times are Unix seconds.
CREATE TABLE sessions (
    token_hash TEXT PRIMARY KEY,
    created_at INTEGER NOT NULL DEFAULT (unixepoch()),
    expires_at INTEGER NOT NULL
) STRICT;
