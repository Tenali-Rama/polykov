#!/usr/bin/env bash
# Browser e2e: Playwright (headless Chromium) against a fresh real server serving web/dist.
# Needs web/node_modules and web/dist (verify's web-install and web-build steps); builds the server.
# Prints "e2e: ok", or "e2e: <reason>" and the first failing test. Extra args go to `playwright test`.
set -uo pipefail
cd "$(git rev-parse --show-toplevel)"

SERVER=target/debug/polykov-server
PASSWORD=e2e-test-password

fail() { echo "e2e: $*" >&2; exit 1; }

cargo build --quiet -p polykov-server || fail "cargo build failed"
[ -f web/dist/index.html ] || fail "web/dist/index.html missing; build the web UI first"
[ -d web/node_modules/@playwright/test ] || fail "web/node_modules missing; run npm --prefix web ci"

# A no-op when this Playwright version's Chromium is already installed.
(cd web && npx playwright install chromium) >/dev/null 2>&1 || fail "playwright install chromium failed"

tmp=$(mktemp -d) || fail "cannot create temp dir"
server_pid=
cleanup() {
  if [ -n "$server_pid" ]; then kill "$server_pid" 2>/dev/null; wait "$server_pid" 2>/dev/null; fi
  rm -rf "$tmp"
}
trap cleanup EXIT

port=$(python3 -c 'import socket; s=socket.socket(); s.bind(("127.0.0.1", 0)); print(s.getsockname()[1])') \
  || fail "cannot find a free port"
base="http://127.0.0.1:$port"

unset POLYKOV_DATA_DIR POLYKOV_BIND POLYKOV_WEB_DIR XDG_DATA_HOME
"$SERVER" --data-dir "$tmp/data" --bind "127.0.0.1:$port" --web-dir web/dist >"$tmp/server.log" 2>&1 &
server_pid=$!

for _ in $(seq 50); do
  kill -0 "$server_pid" 2>/dev/null || fail "server exited: $(tail -n 1 "$tmp/server.log")"
  curl -fsS -o /dev/null "$base/health" 2>/dev/null && break
  sleep 0.1
done

status=$(curl -sS -o "$tmp/body" -w '%{http_code}' -X POST -H 'Content-Type: application/json' \
  -d "{\"password\":\"$PASSWORD\"}" "$base/api/setup") || fail "server not reachable at $base"
[ "$status" = 201 ] || fail "POST /api/setup returned $status: $(cat "$tmp/body")"

if (cd web && BASE_URL="$base" E2E_PASSWORD="$PASSWORD" npx playwright test --max-failures=1 "$@") \
    >"$tmp/playwright.log" 2>&1; then
  tail -n 1 "$tmp/playwright.log"
  echo "e2e: ok"
else
  echo "e2e: failed"
  # The first failure block of the line reporter, else the log tail (e.g. a config error).
  first=$(awk '/^  1\) /{on=1} /^  2\) /{exit} on' "$tmp/playwright.log" | head -n 60)
  if [ -n "$first" ]; then echo "$first"; else tail -n 30 "$tmp/playwright.log"; fi
  exit 1
fi
