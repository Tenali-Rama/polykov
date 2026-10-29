#!/usr/bin/env bash
# One-time (idempotent) machine setup for the factory: tools, git identity, hooks.
set -euo pipefail
cd "$(git rev-parse --show-toplevel)"

if ! command -v gh >/dev/null; then
  sudo mkdir -p -m 755 /etc/apt/keyrings
  curl -fsSL https://cli.github.com/packages/githubcli-archive-keyring.gpg \
    | sudo tee /etc/apt/keyrings/githubcli-archive-keyring.gpg >/dev/null
  echo "deb [arch=$(dpkg --print-architecture) signed-by=/etc/apt/keyrings/githubcli-archive-keyring.gpg] https://cli.github.com/packages stable main" \
    | sudo tee /etc/apt/sources.list.d/github-cli.list >/dev/null
  sudo apt-get update -qq && sudo apt-get install -y -qq gh
fi

command -v jq >/dev/null || sudo apt-get install -y -qq jq
command -v cc >/dev/null || sudo apt-get install -y -qq build-essential pkg-config libssl-dev

if ! command -v cargo >/dev/null && [ ! -x "$HOME/.cargo/bin/cargo" ]; then
  curl -fsSL https://sh.rustup.rs | sh -s -- -y -q --profile minimal -c rustfmt,clippy
fi

git config user.name "Tenali-Rama"
git config user.email "tenalirama.krishna125@gmail.com"
git config core.hooksPath scripts/hooks
chmod +x scripts/*.sh scripts/factory/*.sh scripts/hooks/*
mkdir -p .factory
echo "bootstrap done. Open a new shell (or: source ~/.cargo/env) so cargo is on PATH."
