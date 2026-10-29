# 0001 — Workspace layout and default port

- **Status**: accepted
- **Context**: M0 needs a server and a CLI that share a version and can test against each other.
- **Decision**: One Cargo workspace (root `Cargo.toml`) with members `server/` (package and binary `polykov-server`, lib crate `polykov_server`) and `cli/` (package and binary `polykov`). The version lives in `[workspace.package]`. The server builds its router in `polykov_server::app::router()`, separate from `main`. It listens on `127.0.0.1:7700` by default (`--bind` overrides it). The default may be changed by the owner.
- **Consequences**: Integration tests and the CLI's tests use the lib without binding a port. Keep `main.rs` thin, and don't move the router into the binary. Both binaries always ship the same version.
