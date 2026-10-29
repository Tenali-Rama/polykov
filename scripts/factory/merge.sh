#!/usr/bin/env bash
# The ONLY way code reaches main (agents are blocked from pushing to main directly).
# Squash-merges an approved branch after every gate passes, then deletes the branch.
#   merge.sh <branch> <id> "<title>" <approved-review-file>
set -euo pipefail
cd "$(git rev-parse --show-toplevel)"
die() { echo "NOT MERGED: $1" >&2; exit 1; }
[ $# -eq 4 ] || die "usage: merge.sh <branch> <id> <title> <review-file>"
branch=$1 id=$2 title=$3 review=$4

[ -f "$review" ] || die "review file $review not found"
review=$(realpath "$review")
grep -q '^## Round [0-9]* — APPROVED' "$review" || die "review file has no APPROVED round"

git fetch -q origin
head=$(git rev-parse "origin/$branch") || die "branch $branch not on origin"
status=$(scripts/factory/ci-status.sh "$head" || true)
[ "$status" = pass ] || die "CI is '$status' for $branch@${head:0:7} (wait for pass)"

wt=.factory/wt/merge-$id
git worktree remove --force "$wt" >/dev/null 2>&1 || true
git worktree add -q --detach "$wt" origin/main
cleanup() { git worktree remove --force "$wt" >/dev/null 2>&1 || true; }
trap cleanup EXIT

cd "$wt"
git merge -q --squash "$head" >/dev/null 2>&1 || die "conflicts with main: rebase $branch on origin/main"
mkdir -p docs/reviews
cp "$review" "docs/reviews/$id.md"
git add -A
git commit -q -m "$id: $title"
scripts/verify.sh >/dev/null || die "verify fails after squash onto main (run clean-verify.sh)"
git push -q origin HEAD:main || die "main moved during merge — run merge.sh again"
git push -q origin --delete "$branch" || true
echo "MERGED $id as $(git rev-parse --short HEAD) on main"
