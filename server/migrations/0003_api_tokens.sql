-- Named, non-expiring API tokens (spec §9). Only the SHA-256 hex of each token is stored;
-- times are Unix seconds, and `last_used` stays NULL until the token is first used.
CREATE TABLE api_tokens (
    id         INTEGER PRIMARY KEY,
    name       TEXT NOT NULL,
    token_hash TEXT NOT NULL UNIQUE,
    created_at INTEGER NOT NULL DEFAULT (unixepoch()),
    last_used  INTEGER
) STRICT;
