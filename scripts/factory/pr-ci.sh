#!/usr/bin/env bash
# CI result for a PR's head commit (or for main), via the Actions API.
#   pr-ci.sh <pr-number|main> [--wait]   prints success | failure | pending   (--wait: up to 30 min)
# On failure, see the cause with: gh run view <id> --log-failed | tail -60   (the id is printed too)
set -euo pipefail
if [ "$1" = main ]; then sha=$(git ls-remote origin refs/heads/main | cut -f1)
else sha=$(gh pr view "$1" --json headRefOid -q .headRefOid); fi
tries=1; [ "${2:-}" = --wait ] && tries=120
for ((i = 0; i < tries; i++)); do
  run=$(gh run list --commit "$sha" --workflow ci --limit 1 --json databaseId,status,conclusion -q '.[0] // empty')
  if [ -n "$run" ] && [ "$(jq -r .status <<<"$run")" = completed ]; then
    c=$(jq -r .conclusion <<<"$run")
    echo "$c run=$(jq -r .databaseId <<<"$run")"
    [ "$c" = success ] && exit 0 || exit 1
  fi
  [ $tries -gt 1 ] && sleep 15
done
echo pending
exit 3
