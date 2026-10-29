# cli

`polykov`: the command-line client (clap). Spec §2, §7.

## Layout
- `src/main.rs`: the clap `Cli` struct, dispatch and output/error printing.
- `src/client.rs`: blocking HTTP client for the server API. `src/config.rs`: `cli.toml` and server URL resolution. `src/error.rs`: the error type.
- `src/cmd/`: one file per subcommand, plus `Ctx`/`Output` in `mod.rs`.
- `tests/`: binary tests through `assert_cmd`; `tests/common` runs the real server router on a random port.

## Rules
- `--help` lists only subcommands that are implemented.
- The CLI talks to the server only over its HTTP API, through `client::Client`.
- Every command supports `--json`; errors are one line on stderr with exit 1. See `API.md` for adding a subcommand.
