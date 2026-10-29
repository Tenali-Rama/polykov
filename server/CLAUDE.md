# server

`polykov-server`: Axum + Tokio HTTP server. Spec §2.

## Layout
- `src/lib.rs`: crate `polykov_server`, used by the binary, `tests/`, and the CLI's tests.
- `src/app.rs`: `router()` and the handlers. Add routes here (or in modules it mounts).
- `src/main.rs`: argument parsing and port binding only. No app logic.
- `tests/`: integration tests that drive `app::router()` with `tower::ServiceExt::oneshot`, no port needed.

## Rules
- No `unwrap`/`expect` outside tests.
- Every new endpoint goes in `API.md` and gets an integration test.
