#!/usr/bin/env bash
# Print one section of spec.md (heading line through the next heading of same or higher level).
#   spec.sh "Tasks (GTD)"     match by heading text (case-insensitive substring)
#   spec.sh --list            list all headings
set -euo pipefail
cd "$(git rev-parse --show-toplevel)"
[ $# -ge 1 ] || { echo "usage: spec.sh <heading>|--list" >&2; exit 2; }
if [ "$1" = --list ]; then grep -E '^#{1,4} ' spec.md; exit 0; fi
awk -v q="$(tr '[:upper:]' '[:lower:]' <<<"$1")" '
  /^#+ / {
    lvl = length($1)
    if (on && lvl <= start) exit
    if (!on && index(tolower($0), q)) { on = 1; start = lvl; found = 1 }
  }
  on { print }
  END { exit found ? 0 : 1 }
' spec.md || { echo "spec.sh: no heading matches '$1' (try --list)" >&2; exit 1; }
