#!/usr/bin/env bash
# COO start check: is the factory able to run? One line per check; exits non-zero if any fail.
set -uo pipefail
cd "$(git rev-parse --show-toplevel)"
fail=0
check() {
  local name=$1 fix=$2; shift 2
  if "$@" >/dev/null 2>&1; then echo "OK   $name"; else echo "FAIL $name — fix: $fix"; fail=1; fi
}

export GH_REPO=${GH_REPO:-Tenali-Rama/polykov}
check "gh installed"        "run scripts/factory/bootstrap.sh"          command -v gh
check "gh authenticated"    "owner: gh auth login --with-token (fine-grained PAT)" gh auth status
check "gh repo access"      "token needs access to $GH_REPO"            gh repo view "$GH_REPO"
check "main protected"      "owner: scripts/factory/protect-main.sh"    gh api "repos/$GH_REPO/branches/main/protection"
check "cargo installed"     "run scripts/factory/bootstrap.sh"          command -v cargo
check "node installed"      "install Node 22"                           command -v node
check "jq installed"        "run scripts/factory/bootstrap.sh"          command -v jq
check "git push access"     "deploy key with write access on GitHub"    git ls-remote origin
check "commit identity"     "run scripts/factory/bootstrap.sh"          test "$(git config user.email)" = tenalirama.krishna125@gmail.com
check "commit hook active"  "run scripts/factory/bootstrap.sh"          test "$(git config core.hooksPath)" = scripts/hooks
check "on main, clean tree" "commit/stash local changes, git switch main" \
  bash -c '[ "$(git rev-parse --abbrev-ref HEAD)" = main ] && [ -z "$(git status --porcelain)" ]'
check "main up to date"     "git pull --ff-only"                        git pull --ff-only -q
check "board valid"         "fix task frontmatter (board.sh --check)"   scripts/factory/board.sh --check
mkdir -p .factory
exit $fail
