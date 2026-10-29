#!/usr/bin/env bash
# CI result for a commit, read via git (no GitHub API/token needed).
# CI publishes refs/ci/pass/<sha> or refs/ci/fail/<sha> when it finishes.
#   ci-status.sh <sha|ref> [--wait]   prints pass | fail | pending   (--wait: up to 25 min)
set -euo pipefail
cd "$(git rev-parse --show-toplevel)"
git fetch -q origin 2>/dev/null || true
sha=$(git rev-parse "$1^{commit}")
tries=1; [ "${2:-}" = --wait ] && tries=50
for ((i = 0; i < tries; i++)); do
  refs=$(git ls-remote origin "refs/ci/*/$sha")
  if grep -q "refs/ci/fail/$sha" <<<"$refs"; then echo fail; exit 1; fi
  if grep -q "refs/ci/pass/$sha" <<<"$refs"; then echo pass; exit 0; fi
  [ $tries -gt 1 ] && sleep 30
done
echo pending
exit 3
