# 0005 — API tokens

- **Status**: accepted
- **Context**: Spec §9 lets clients and scripts authenticate with API tokens as well as sessions (task 005), through the one `auth::Session` extractor.
- **Decision**: API tokens are made like session tokens (32 random bytes, hex) and stored only as SHA-256 hex in their own `api_tokens` table with a name, `created_at` and `last_used`. They never expire. The extractor looks a bearer/cookie token up in `sessions` first, then in `api_tokens`, and a hit there updates `last_used`. `Session.expires_at` is `None` for an API token.
- **Consequences**: Routes need no change to accept API tokens. Revocation is only `DELETE /api/tokens/{id}`; logout does not touch API tokens. Adding expiry or scopes later means new columns plus checks in `tokens::authenticate`; keep the hash-only storage.
