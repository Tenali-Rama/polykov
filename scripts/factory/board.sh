#!/usr/bin/env bash
# Task board built from task-file frontmatter (docs/tasks/NNN-*.md).
#   board.sh          all tasks
#   board.sh --ready  todo tasks whose deps are all done
#   board.sh --check  validate frontmatter (used by verify.sh)
#   board.sh --overlap <id>...  print task pairs whose touches overlap; exit 1 if any (COO dispatch)
set -euo pipefail
cd "$(git rev-parse --show-toplevel)"
mode=${1:-all}

field() { sed -n '/^---$/,/^---$/p' "$2" | sed -n "s/^$1:[[:space:]]*//p" | head -1; }
list() { tr -d '[]"' <<<"$1" | tr ',' '\n' | sed 's/^ *//;s/ *$//' | sed '/^$/d'; }
# Shared files never block parallel work: their rebase conflicts are mechanical (LESSONS.md, Rebase).
shared() { [[ $1 == Cargo.lock || $1 == API.md || $1 == */API.md || $1 == docs/decisions/* ]]; }
# Two touches overlap if equal, or if one is a dir (trailing /) that contains the other.
overlaps() { [[ $1 == "$2" || ($1 == */ && $2 == "$1"*) || ($2 == */ && $1 == "$2"*) ]]; }

shopt -s nullglob
files=(docs/tasks/[0-9]*.md)
declare -A status touch
for f in "${files[@]}"; do
  id=$(field id "$f"); status[$id]=$(field status "$f"); touch[$id]=$(field touches "$f")
done

if [ "$mode" = --overlap ]; then
  shift; ids=("$@"); hit=0
  for id in "${ids[@]}"; do [ -n "${touch[$id]+x}" ] || { echo "unknown task '$id'"; exit 2; }; done
  for ((i = 0; i < ${#ids[@]}; i++)); do
    for ((j = i + 1; j < ${#ids[@]}; j++)); do
      for p in $(list "${touch[${ids[i]}]}"); do
        for q in $(list "${touch[${ids[j]}]}"); do
          # A trailing + marks an append-only edit (one registration line); two of those never clash.
          [[ $p == *+ && $q == *+ ]] && continue
          if ! shared "${p%+}" && ! shared "${q%+}" && overlaps "${p%+}" "${q%+}"; then
            echo "${ids[i]} ${ids[j]}: $p ~ $q"; hit=1
          fi
        done
      done
    done
  done
  [ $hit = 0 ] && echo "no overlap"
  exit $hit
fi

errors=0
for f in "${files[@]}"; do
  id=$(field id "$f"); st=$(field status "$f"); deps=$(field deps "$f")
  if [ "$mode" = --check ]; then
    for k in id title milestone status deps touches spec; do
      [ -n "$(field "$k" "$f")" ] || { echo "$f: missing '$k'"; errors=1; }
    done
    case $st in todo|done|dropped) ;; *) echo "$f: bad status '$st'"; errors=1 ;; esac
    for d in $(list "$deps"); do
      [ -n "${status[$d]+x}" ] || { echo "$f: unknown dep '$d'"; errors=1; }
    done
    while IFS= read -r h; do
      scripts/factory/spec.sh "$h" >/dev/null 2>&1 || { echo "$f: spec heading not found: $h"; errors=1; }
    done < <(field spec "$f" | grep -o '"[^"]*"' | tr -d '"')
    continue
  fi
  ready=no
  if [ "$st" = todo ]; then
    ready=yes
    for d in $(list "$deps"); do [ "${status[$d]:-}" = done ] || ready=no; done
  fi
  [ "$mode" = --ready ] && [ $ready = no ] && continue
  printf '%s | %-7s | %s | ready=%s | deps=%s | touches=%s | %s\n' \
    "$id" "$st" "$(field milestone "$f")" "$ready" "$(list "$deps" | paste -sd, -)" \
    "$(list "$(field touches "$f")" | paste -sd, -)" "$(field title "$f")"
done
exit $errors
