# cli API

## Commands
Global flags (accepted before or after the subcommand):
- `--server <URL>`: server base URL. First match wins: `--server`, `POLYKOV_SERVER` (empty = unset), `server` in the config file, `http://127.0.0.1:7700`. Must start with `http://` or `https://`; a trailing `/` is dropped.
- `--json`: print one JSON object on stdout instead of text.

| Command | Does | `--json` output |
|---|---|---|
| `polykov login` | Password from `POLYKOV_PASSWORD`, else a no-echo prompt on the tty. `POST /api/login`, then writes the server URL and token to the config file. | `{"server":"...","logged_in":true}` |
| `polykov status` | `GET /health`, then `GET /api/me` if a token is stored for this server. Exit 0 whenever the server answers, even with an invalid token. | `{"server","version","db","token":"valid"\|"invalid"\|"none","session_expires":"<RFC 3339>"\|null}` |
| `polykov logout` | `POST /api/logout`, then removes the token (keeps `server`). On `401` the token is removed too, but it exits 1. Without a stored token: exit 1. | `{"server":"...","logged_out":true}` |

`polykov --version` prints `polykov <crate version>`. Unknown subcommands and bad arguments exit 2 with a clap error.

### Errors
Any failure prints one line on stderr and exits 1: `polykov: <message>`, or `{"error":"<message>"}` with `--json`. Stdout stays empty. Messages:
- `cannot reach server at <url>: <cause>`: connection refused, DNS, TLS, or the 10 s timeout.
- `session missing or expired; run polykov login`: the server answered `401` to an authenticated call.
- `server returned <status>[: <error field>]`: any other unexpected status.

### Config file
`$XDG_CONFIG_HOME/polykov/cli.toml` (absolute `XDG_CONFIG_HOME` only), else `$HOME/.config/polykov/cli.toml`:
```toml
server = "http://127.0.0.1:7700"
token = "<64 hex chars>"
```
Created with mode `0600`; an existing file keeps the mode its owner set. The token is sent only to the server it was issued by: with another `--server`/`POLYKOV_SERVER`, commands run unauthenticated.

## Code (binary crate, `src/`)
- `client::Client::new(server, token: Option<&str>)`: blocking HTTP client (`ureq`, 10 s timeout) that sends `Authorization: Bearer <token>` when given one.
  - `get(path)` / `post(path, body: Option<&Value>)` → `Result<Reply, Error>`. Every HTTP status is a `Reply { status, body }` (`body` is `Null` if not JSON); only transport failures are `Error::Unreachable`.
  - `Reply::ok()` → the body of a `2xx`, `Error::Unauthorized` on `401`, else `Error::Server`.
- `error::Error`: `Unreachable`, `Unauthorized`, `Server`, `Msg(String)`; `Display` is the user-facing line.
- `config`: `CliConfig { server, token }` with `load`/`save`/`token_for(server)`, `path(env)`, `resolve_server(flag, env, file)`, `env(name)`.
- `cmd::Ctx { server, config, config_path }`: built once in `main`; `ctx.client()` is authenticated when a token is stored for `ctx.server`. `cmd::Output { human, json }` is what a command returns; `main` prints the one `--json` selects.

### Adding a subcommand
1. Create `src/cmd/<name>.rs` with `pub fn run(ctx: &Ctx, ...) -> Result<Output, Error>`. Call the server via `ctx.client()`, and use `reply.ok()?` unless a status needs special handling.
2. Add `pub mod <name>;` to `src/cmd/mod.rs`, a variant (with a doc comment, used as its help) to `Command` in `src/main.rs`, and its match arm.
3. Add an integration test in `tests/<name>.rs` using `tests/common` (`Server::start`, `Home::polykov`, `run`), the command to the `--help` list test in `tests/cli.rs`, and a row to the table above.
