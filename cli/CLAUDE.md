# cli

`polykov`: the command-line client (clap). Spec §2, §7.

## Layout
- `src/main.rs`: the clap `Cli` struct and entry point.
- `tests/`: binary tests through `assert_cmd`.

## Rules
- `--help` lists only subcommands that are implemented.
- The CLI talks to the server only over its HTTP API (task 009 adds the client).
