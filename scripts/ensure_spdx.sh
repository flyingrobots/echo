#!/usr/bin/env bash
# SPDX-License-Identifier: Apache-2.0
set -euo pipefail
ROOT=$(git rev-parse --show-toplevel)
cd "$ROOT"
CODE_HEADER='SPDX-License-Identifier: Apache-2.0'
DUAL_HEADER='SPDX-License-Identifier: Apache-2.0 OR MIND-UCAL-1.0'
modified=0
is_dual(){
  case "$1" in
    docs/*|rmg-math/*|*.tex|*.sty|*.md) return 0;;
    *) return 1;;
  esac
}
skip(){
  case "$1" in
    target/*|node_modules/*|vendor/*|docs/benchmarks/vendor/*|*.png|*.svg|*.pdf|*.wasm|*.woff*|*.map|*.ico) return 0;;
    *) return 1;;
  esac
}
add_header(){
  local f="$1" header="$2"
  # keep shebang if present
  local first
  first=$(head -n1 "$f" || true)
  local tmp
  tmp=$(mktemp)
  if [[ "$first" =~ ^#! ]]; then
    echo "$first" > "$tmp"
    echo "$header" >> "$tmp"
    tail -n +2 "$f" >> "$tmp"
  else
    echo "$header" > "$tmp"
    cat "$f" >> "$tmp"
  fi
  mv "$tmp" "$f"
  modified=1
}
STAGED=$(git diff --cached --name-only)
[[ -z "$STAGED" ]] && exit 0
while IFS= read -r f; do
  [[ -f "$f" ]] || continue
  skip "$f" && continue
  head5=$(head -n5 "$f")
  if is_dual "$f"; then
    if ! grep -q "$DUAL_HEADER" <<<"$head5"; then
      add_header "$f" "$DUAL_HEADER"
    fi
  else
    if ! grep -q "$CODE_HEADER" <<<"$head5"; then
      add_header "$f" "$CODE_HEADER"
    fi
  fi
  # restage if we modified
  if [[ $modified -eq 1 ]]; then
    git add "$f"
  fi
done <<< "$STAGED"
if [[ $modified -eq 1 ]]; then
  echo "pre-commit: inserted SPDX headers; review & commit again" >&2
  exit 1
fi
