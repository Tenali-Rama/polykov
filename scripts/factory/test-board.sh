#!/usr/bin/env bash
# Fixture tests for board.sh --overlap and the --check trailing-slash rule (run by verify.sh).
set -euo pipefail
src=$(cd "$(dirname "$0")" && pwd)/board.sh
tmp=$(mktemp -d)
trap 'rm -rf "$tmp"' EXIT
cd "$tmp"
git init -q .
mkdir -p docs/tasks server/src/a scripts/factory
cp "$src" scripts/factory/board.sh
task() { printf -- '---\nid: %s\nstatus: todo\ndeps: []\ntouches: [%s]\n---\n' "$1" "$2" >"docs/tasks/$1-t.md"; }
fail=0
expect() { # expect <exit code> <task ids...>
  local want=$1 got=0
  shift
  scripts/factory/board.sh --overlap "$@" >/dev/null || got=$?
  if [ "$got" != "$want" ]; then echo "FAIL --overlap $*: exit $got, want $want"; fail=1; fi
}

task 101 server/src/lib.rs
task 102 server/src/
expect 1 101 102 # dir contains file
task 103 server/src
expect 1 101 103 # dir written without trailing /
task 104 server/src/a/
task 105 server/src/ab/
expect 0 104 105 # sibling with a common prefix
task 106 server/src/lib.rs+
task 107 server/src/lib.rs+
expect 0 106 107 # two append-only entries
expect 1 101 106 # append-only vs normal edit
task 108 'Cargo.lock, docs/decisions'
task 109 'Cargo.lock, docs/decisions/0009-x.md'
expect 0 108 109 # shared files, with and without trailing /
task 110 server/API.md
task 111 server/API.md
expect 0 110 111 # shared API.md
task 112 web/x.ts
expect 0 101 112 # unrelated
expect 2 101 999 # unknown id

out=$(scripts/factory/board.sh --check 2>&1 || true)
if [[ $out != *"touches dir 'server/src' needs a trailing /"* ]]; then
  echo "FAIL --check did not flag 'server/src'"
  fail=1
fi
[ $fail = 0 ] && echo "board.sh: all overlap cases pass"
exit $fail
