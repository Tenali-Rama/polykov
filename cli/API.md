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
| `polykov new <type> <title>... [--tag T]... [--prop K=V]...` | `POST /api/objects`; title words are joined with spaces. Prints `created <id>`. | the object |
| `polykov show <id>` | `GET /api/objects/{id}`. Prints the title, then aligned rows `id`, `type`, `tags`, `pinned`, `archived`, `created`, `updated` and one `.<key>` row per property. | the object |
| `polykov ls [--type T] [--tag T] [--pinned] [--archived\|--all] [--limit N]` | `GET /api/objects` with those filters (`--all` = `archived=any`). One line per object, `<id>  <type>  <title>  #tag ...`, the id prefixed with `*` if pinned; then `(n of total shown)` if paged, or `no objects`. | `{"objects":[...],"total":n}` |
| `polykov edit <id> [--title T] [--add-tag T]... [--rm-tag T]... [--prop K=V]... [--unset K]...` | `GET`, merges the tags (`--rm-tag` is normalized like a tag) and properties, then one `PATCH` with only the changed fields. Prints `updated <id>`. | the object |
| `polykov rm <id>` | `DELETE /api/objects/{id}` (to the trash). Prints `trashed <id>`. | `{"id":n,"trashed":true}` |
| `polykov pin\|unpin\|archive\|unarchive <id>` | `PATCH` of `pinned`/`archived`. Prints e.g. `pinned <id>`, `unarchived <id>`. | the object |
| `polykov link add\|rm <id> <to>` | `PUT`/`DELETE /api/objects/{id}/links/{to}`. Prints `linked <id> -> <to>` / `unlinked <id> -> <to>`. | `{"id":n,"to":n,"linked":true}` / `"unlinked":true` |
| `polykov link ls <id>` | `GET /api/objects/{id}/links`. Prints `links:` and `backlinks:`, each followed by indented `<id>  <type>  <title>` lines, or `  none`. | the server body `{"links":[...],"backlinks":[...]}` |
| `polykov comment add <id> <text>...` | `POST /api/objects/{id}/comments`; text words are joined with spaces. Prints `added comment <cid>`. | the comment |
| `polykov comment ls <id>` | `GET /api/objects/{id}/comments`. Per comment, oldest first: `#<cid> <created>`, then the body with each line indented two spaces; or `no comments`. | the server body `{"comments":[...]}` |
| `polykov comment edit <cid> <text>...` | `PATCH /api/comments/{cid}`. Prints `updated comment <cid>`. | the comment |
| `polykov comment rm <cid>` | `DELETE /api/comments/{cid}`. Prints `deleted comment <cid>`. | `{"id":n,"deleted":true}` |
| `polykov trash ls` | `GET /api/trash`. One line per object, newest deleted first: `<id>  <type>  <title>  deleted <YYYY-MM-DD>  purge <YYYY-MM-DD>`; or `trash is empty`. | the server body `{"objects":[...]}` |
| `polykov trash restore <id>` | `POST /api/trash/{id}/restore`. Prints `restored <id>`. | the object |
| `polykov trash purge <id>` | `DELETE /api/trash/{id}` (for good). Prints `purged <id>`. | `{"id":n,"purged":true}` |
| `polykov trash empty --yes` | `DELETE /api/trash`. Prints `purged <n>`. Without `--yes`: clap error naming `--yes`, exit 2, no request. | `{"purged":n}` |

`--prop K=V`: `V` becomes an integer or float, then `true`/`false`, else a string; a missing `=` or empty key exits 2. `<id>` must be a non-negative integer (else exit 2). Object errors from the server print as below, e.g. `polykov: server returned 404: object not found`.

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
  - `get(path)` / `delete(path)` / `post(path, body: Option<&Value>)` / `put(path, body: Option<&Value>)` / `patch(path, body: &Value)` → `Result<Reply, Error>`. `path` includes any query string, already percent-encoded. Every HTTP status is a `Reply { status, body }` (`body` is `Null` if not JSON); only transport failures are `Error::Unreachable`.
  - `Reply::ok()` → the body of a `2xx`, `Error::Unauthorized` on `401`, else `Error::Server`.
- `error::Error`: `Unreachable`, `Unauthorized`, `Server`, `Msg(String)`; `Display` is the user-facing line.
- `config`: `CliConfig { server, token }` with `load`/`save`/`token_for(server)`, `path(env)`, `resolve_server(flag, env, file)`, `env(name)`.
- `cmd::Ctx { server, config, config_path }`: built once in `main`; `ctx.client()` is authenticated when a token is stored for `ctx.server`. `cmd::Output { human, json }` is what a command returns; `main` prints the one `--json` selects.
- `cmd::object`: one file per object command (`new`, `show`, `ls`, `edit`, `rm`, `flag` for pin/archive) plus the shared rendering and `parse_prop` (the clap value parser for `--prop`) in `mod.rs`; `object::summary` (`<id>  <type>  <title>`) and `object::text` are reused by the other groups. Commands with flags take a `clap::Args` struct (`new::Args`, ...) that `main` embeds in its `Command` variant.
- `cmd::link`, `cmd::comment`, `cmd::trash`: one file per command group, each a clap `Cmd` subcommand enum (one `#[command(subcommand)]` variant in `main`) and `run(ctx, cmd)`.

### Adding a subcommand
1. Create `src/cmd/<name>.rs` with `pub fn run(ctx: &Ctx, ...) -> Result<Output, Error>`. Call the server via `ctx.client()`, and use `reply.ok()?` unless a status needs special handling.
2. Add `pub mod <name>;` to `src/cmd/mod.rs`, a variant (with a doc comment, used as its help) to `Command` in `src/main.rs`, and its match arm.
3. Add an integration test in `tests/<name>.rs` using `tests/common` (`Server::start`, `Home::polykov`, `run`), the command to the `--help` list test in `tests/cli.rs`, and a row to the table above.
