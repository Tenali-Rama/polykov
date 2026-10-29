#!/usr/bin/env bash
# End-to-end smoke test: the real server and CLI binaries together, against the built web UI.
# Needs target/debug/{polykov-server,polykov} (built here) and web/dist (built by verify's web-build step).
# Exits non-zero on the first failed check with one line: "smoke: <reason>".
set -uo pipefail
cd "$(git rev-parse --show-toplevel)"

SERVER=target/debug/polykov-server
CLI=target/debug/polykov
PASSWORD=smoke-test-password

fail() { echo "smoke: $*" >&2; exit 1; }

cargo build --quiet -p polykov-server -p polykov || fail "cargo build failed"
[ -f web/dist/index.html ] || fail "web/dist/index.html missing; build the web UI first"

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

# Isolate the CLI config and keep env settings from leaking into the server.
export HOME="$tmp/home" XDG_CONFIG_HOME="$tmp/config" POLYKOV_SERVER="$base"
unset POLYKOV_DATA_DIR POLYKOV_BIND POLYKOV_WEB_DIR XDG_DATA_HOME

"$SERVER" --data-dir "$tmp/data" --bind "127.0.0.1:$port" --web-dir web/dist >"$tmp/server.log" 2>&1 &
server_pid=$!

# Prints the HTTP status; the body goes to $tmp/body.
req() { curl -sS -o "$tmp/body" -w '%{http_code}' "$@"; }

for _ in $(seq 50); do
  kill -0 "$server_pid" 2>/dev/null || fail "server exited: $(tail -n 1 "$tmp/server.log")"
  [ "$(req "$base/health" 2>/dev/null)" = 200 ] && break
  sleep 0.1
done

[ "$(req "$base/health")" = 200 ] || fail "/health did not return 200"
jq -e '.status == "ok" and .db == "ok"' "$tmp/body" >/dev/null || fail "/health not ok: $(cat "$tmp/body")"

[ "$(req "$base/")" = 200 ] || fail "/ did not return 200"
cmp -s "$tmp/body" web/dist/index.html || fail "/ did not return web/dist/index.html"

status=$(req -X POST -H 'Content-Type: application/json' -d "{\"password\":\"$PASSWORD\"}" "$base/api/setup")
[ "$status" = 201 ] || fail "POST /api/setup returned $status, expected 201"

POLYKOV_PASSWORD="$PASSWORD" "$CLI" login >"$tmp/login.out" 2>&1 || fail "polykov login failed: $(tail -n 1 "$tmp/login.out")"

"$CLI" status --json >"$tmp/status.json" 2>&1 || fail "polykov status failed: $(tail -n 1 "$tmp/status.json")"
jq -e '.token == "valid"' "$tmp/status.json" >/dev/null || fail "polykov status token not valid: $(cat "$tmp/status.json")"

session=$(sed -n 's/^token = "\(.*\)"$/\1/p' "$tmp/config/polykov/cli.toml" 2>/dev/null)
[ -n "$session" ] || fail "no token in CLI config after login"

status=$(req -X POST -H "Authorization: Bearer $session" -H 'Content-Type: application/json' \
  -d '{"name":"smoke"}' "$base/api/tokens")
[ "$status" = 201 ] || fail "POST /api/tokens returned $status, expected 201"
api_token=$(jq -r '.token' "$tmp/body")

[ "$(req -H "Authorization: Bearer $api_token" "$base/api/me")" = 200 ] || fail "/api/me with API token not 200"
jq -e '.owner == true and .session_expires == null' "$tmp/body" >/dev/null \
  || fail "/api/me with API token: unexpected body $(cat "$tmp/body")"

echo "smoke: ok"
