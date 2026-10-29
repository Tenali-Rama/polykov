# Installing Polykov

Polykov is one server process (`polykov-server`) that also serves the web UI. All data lives in one data dir: `polykov.db`, `attachments/`, `models/`, `backups/` and `config.toml`.

## Docker Compose (recommended)

Needs Docker with the Compose plugin. From a checkout of this repo:

```sh
docker compose up -d
curl -s http://localhost:7700/health   # {"status":"ok",...}
```

Then open `http://localhost:7700`. The data dir is the named volume `polykov-data`, mounted at `/data`, so it survives restarts, upgrades and `docker compose down` (but not `docker compose down -v`).

- Upgrade: `git pull && docker compose up -d --build`. Migrations run on start.
- Logs: `docker compose logs -f polykov`.
- The container runs as the non-root user `polykov` (uid 10001) and reports its health from `/health`.
- The image sets `POLYKOV_DATA_DIR=/data`, `POLYKOV_BIND=0.0.0.0:7700` and `POLYKOV_WEB_DIR=/app/web`. Env vars override `config.toml`, so change these with `environment:` in `docker-compose.yml`, not in the file.
- To use another host port, change the left side of `"7700:7700"`. To keep it off the network, use `"127.0.0.1:7700:7700"`.

Plain Docker, without Compose:

```sh
docker build -t polykov .
docker run -d --name polykov -p 7700:7700 -v polykov-data:/data --restart unless-stopped polykov
```

## Single binary

Needs a Rust toolchain (stable) and Node.js 22. Build the web UI and the server:

```sh
npm --prefix web ci && npm --prefix web run build
cargo build --release -p polykov-server
```

Run it from the repo root, or pass `--web-dir` with the path to `web/dist`:

```sh
./target/release/polykov-server --web-dir web/dist
```

It listens on `127.0.0.1:7700` by default and keeps its data in `$XDG_DATA_HOME/polykov` (or `~/.local/share/polykov`). Override with `--bind`/`POLYKOV_BIND`, `--data-dir`/`POLYKOV_DATA_DIR` or `config.toml` in the data dir; flags beat env vars, which beat the file. `polykov-server --help` lists every option.

## Your responsibilities

- **HTTPS**: Polykov speaks plain HTTP. Put it behind a reverse proxy that terminates TLS (Caddy, nginx, Traefik) before exposing it beyond your machine, and forward to `127.0.0.1:7700`. Without HTTPS, your password and session cookie cross the network in clear text.
- **Encryption at rest**: Polykov does not encrypt its data. Keep the data dir (or the Docker volume) on an encrypted disk, such as LUKS or an encrypted cloud volume.
- **Backups**: copy the data dir somewhere safe until built-in backups land.
