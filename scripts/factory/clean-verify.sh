#!/usr/bin/env bash
# Run verify.sh on a pushed ref in a brand-new worktree: only committed files, no local leftovers.
# This is the local stand-in for CI and shows full logs, which CI results don't.
#   clean-verify.sh <ref> [--keep]   (--keep leaves the worktree at .factory/wt/<name> for more checks)
set -uo pipefail
cd "$(git rev-parse --show-toplevel)"
git fetch -q origin
ref=$1
wt=.factory/wt/clean-$(tr '/' '-' <<<"$ref")
git worktree remove --force "$wt" >/dev/null 2>&1
git worktree add -q --detach "$wt" "$ref" || exit 1
(cd "$wt" && scripts/verify.sh)
rc=$?
if [ "${2:-}" = --keep ]; then echo "worktree kept: $wt"; else git worktree remove --force "$wt"; fi
exit $rc
