#!/usr/bin/env bash
# The single verification entry point, identical locally and in CI.
# Prints one line per step; on failure, the tail of that step's log. Full logs: .factory/logs/
set -uo pipefail
cd "$(git rev-parse --show-toplevel)"
LOG_DIR=.factory/logs
mkdir -p "$LOG_DIR"
TAIL=${VERIFY_TAIL:-40}
failed=0

step() {
  local name=$1; shift
  local log="$LOG_DIR/$name.log"
  if "$@" >"$log" 2>&1; then
    echo "PASS $name"
  else
    echo "FAIL $name  (full log: $log)"
    tail -n "$TAIL" "$log" | sed 's/^/    /'
    failed=1
  fi
}

step tasks scripts/factory/board.sh --check
step board-test scripts/factory/test-board.sh
step size scripts/factory/check-size.sh

if [ -f Cargo.toml ]; then
  step fmt cargo fmt --all -- --check
  step clippy cargo clippy --workspace --all-targets --quiet -- -D warnings
  step test cargo test --workspace --quiet
fi

if [ -f web/package.json ]; then
  [ -d web/node_modules ] || step web-install npm --prefix web ci --no-audit --no-fund
  for s in lint check test build; do
    if jq -e ".scripts.\"$s\"" web/package.json >/dev/null; then
      step "web-$s" npm --prefix web run --silent "$s"
    fi
  done
fi

if [ -f Cargo.toml ] && [ -f web/package.json ]; then
  step smoke scripts/smoke.sh
  step e2e scripts/e2e.sh
fi

if command -v gitleaks >/dev/null; then
  step secrets gitleaks dir --no-banner --redact .
fi

if [ $failed = 0 ]; then echo "VERIFY: PASS"; else echo "VERIFY: FAIL"; fi
exit $failed
