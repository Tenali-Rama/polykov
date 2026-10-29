#!/usr/bin/env bash
# Run once (needs a token with Administration: write): main accepts changes only through PRs with green CI.
set -euo pipefail
repo=${GH_REPO:-Tenali-Rama/polykov}
gh api -X PUT "repos/$repo/branches/main/protection" --input - >/dev/null <<'EOF'
{
  "required_status_checks": { "strict": true, "contexts": ["verify", "policy"] },
  "enforce_admins": true,
  "required_pull_request_reviews": null,
  "restrictions": null,
  "allow_force_pushes": false,
  "allow_deletions": false
}
EOF
echo "main is protected: PR + green CI (verify, policy) required."
