# server API

## Binary
`polykov-server [--bind <addr>]`: listens on `127.0.0.1:7700` by default.

## Library (`polykov_server`)
- `app::router() -> axum::Router`: the full HTTP app, with no state and no bound port.

## HTTP
### `GET /health`
Public. Always `200`:
```json
{"status":"ok","version":"<crate version>"}
```

Unknown paths return `404` with an empty body.
