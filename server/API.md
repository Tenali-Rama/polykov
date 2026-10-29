# server API

## Binary
`polykov-server [--data-dir <dir>] [--bind <addr>] [--web-dir <dir>]`

On start it resolves the data dir, creates `attachments/`, `models/` and `backups/` in it, writes a fully commented `config.toml` if none exists (an existing one is never touched), then reads its settings. Any config error exits non-zero with a one-line message naming the file and line, key, or env var at fault.

### Data dir
First match wins: `--data-dir`, `POLYKOV_DATA_DIR`, `$XDG_DATA_HOME/polykov` (absolute `XDG_DATA_HOME` only), `$HOME/.local/share/polykov`.

### Settings
Precedence: flag > env var > `<data dir>/config.toml` > default. An empty env var counts as unset. An unknown key in `config.toml` is an error.

| Key | Flag | Env | Default | Meaning |
|---|---|---|---|---|
| `bind` | `--bind` | `POLYKOV_BIND` | `127.0.0.1:7700` | Socket address to listen on |
| `web_dir` | `--web-dir` | `POLYKOV_WEB_DIR` | `web/dist` | Built web UI dir; relative paths resolve against the working dir |

## Library (`polykov_server`)
- `app::router() -> axum::Router`: the full HTTP app, with no state and no bound port.
- `config::load(Flags, env: impl Fn(&str) -> Option<String>) -> Result<Config, config::Error>`: prepares the data dir and merges all sources. `Config { data_dir, bind, web_dir }`; `Flags` holds the same fields as `Option`s. `config::Error` displays a one-line message.

## HTTP
### `GET /health`
Public. Always `200`:
```json
{"status":"ok","version":"<crate version>"}
```

Unknown paths return `404` with an empty body.
