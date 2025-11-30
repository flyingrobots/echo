#!/usr/bin/env bash
# SPDX-License-Identifier: Apache-2.0
# © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
set -euo pipefail
# SPDX-License-Identifier: Apache-2.0
# Simple SPDX header check (staged files only).
# Apache-2.0 for code; Apache-2.0 OR MIND-UCAL-1.0 for docs/math.
# Excludes vendor/target/node_modules/coverage assets.

ROOT=$(git rev-parse --show-toplevel)
cd "$ROOT"

CODE_HEADER='SPDX-License-Identifier: Apache-2.0'
DUAL_HEADER='SPDX-License-Identifier: Apache-2.0 OR MIND-UCAL-1.0'
fail=0

is_dual() {
  case "$1" in
    docs/*|rmg-math/*|*.tex|*.sty|*.md) return 0;;
    *) return 1;;
  esac
}

check_file() {
  local f="$1"
  # skip binaries/targets/vendors
  case "$f" in
    target/*|node_modules/*|vendor/*|docs/benchmarks/vendor/*|*.png|*.svg|*.pdf|*.wasm|*.woff*|*.map|*.ico) return 0;;
  esac
  # file might be deleted
  [[ -f "$f" ]] || return 0
  local head
  head=$(head -n 5 "$f")
  if is_dual "$f"; then
    grep -q "$DUAL_HEADER" <<<"$head" || { echo "[SPDX] missing dual header: $f"; fail=1; }
  else
    grep -q "$CODE_HEADER" <<<"$head" || { echo "[SPDX] missing code header: $f"; fail=1; }
  fi
}

STAGED=$(git diff --cached --name-only)
[[ -z "$STAGED" ]] && exit 0

while IFS= read -r f; do
  check_file "$f"
done <<< "$STAGED"

exit $fail
