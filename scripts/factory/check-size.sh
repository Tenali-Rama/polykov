#!/usr/bin/env bash
# Fail if any tracked source file exceeds MAX lines (keeps files small enough to load cheaply).
set -euo pipefail
cd "$(git rev-parse --show-toplevel)"
MAX=${MAX_LINES:-400}
bad=0
while IFS= read -r f; do
  [ -f "$f" ] || continue
  case $f in */generated/*|*/migrations/*) continue ;; esac
  n=$(wc -l <"$f")
  if [ "$n" -gt "$MAX" ]; then echo "$f: $n lines (max $MAX)"; bad=1; fi
done < <(git ls-files --cached --others --exclude-standard -- '*.rs' '*.ts' '*.svelte' '*.kt' '*.js')
exit $bad
