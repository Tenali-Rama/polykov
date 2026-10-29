# Polykov server + web UI. Build: `docker build -t polykov .`  Run: see docs/install.md.

# --- web UI: web/dist ---
FROM node:22-slim AS web
WORKDIR /src/web
COPY web/package.json web/package-lock.json ./
RUN npm ci --no-audit --no-fund
COPY web/ ./
RUN npm run build

# --- server: release binary ---
FROM rust:1-slim-trixie AS server
WORKDIR /src
COPY Cargo.toml Cargo.lock ./
COPY server/ server/
COPY cli/ cli/
COPY parse/ parse/
RUN cargo build --release --locked -p polykov-server

# --- runtime (no curl: the health check uses bash's /dev/tcp to stay small) ---
FROM debian:trixie-slim
RUN useradd --system --uid 10001 --home-dir /data --shell /usr/sbin/nologin polykov \
    && mkdir /data \
    && chown polykov:polykov /data
COPY --from=server /src/target/release/polykov-server /usr/local/bin/polykov-server
COPY --from=web /src/web/dist /app/web
ENV POLYKOV_DATA_DIR=/data \
    POLYKOV_BIND=0.0.0.0:7700 \
    POLYKOV_WEB_DIR=/app/web
USER polykov
VOLUME /data
EXPOSE 7700
HEALTHCHECK --interval=30s --timeout=5s --start-period=10s --retries=3 \
    CMD bash -c 'exec 3<>/dev/tcp/127.0.0.1/7700 && printf "GET /health HTTP/1.0\r\n\r\n" >&3 && grep -q "\"status\":\"ok\"" <&3'
ENTRYPOINT ["/usr/local/bin/polykov-server"]
