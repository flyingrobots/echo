#!/usr/bin/env bash
# SPDX-License-Identifier: Apache-2.0
# © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
set -euo pipefail

# Ban global state patterns in Echo/WARP core crates.
# Goal: no hidden wiring, no init-order dependency, no mutable process-wide state.
#
# Allowed:
#   - const
#   - immutable static data (e.g. magic bytes, lookup tables)
#
# Forbidden:
#   - OnceLock/LazyLock/lazy_static/once_cell/thread_local/static mut
#   - "install_*" global init patterns (heuristic)
#
# Usage:
#   ./scripts/ban-globals.sh
#
# Optional env:
#   BAN_GLOBALS_PATHS="crates/warp-core crates/warp-wasm crates/echo-wasm-abi"
#   BAN_GLOBALS_ALLOWLIST=".ban-globals-allowlist"

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"

PATHS_DEFAULT="crates/warp-core crates/warp-wasm crates/echo-wasm-abi"
PATHS="${BAN_GLOBALS_PATHS:-$PATHS_DEFAULT}"

ALLOWLIST="${BAN_GLOBALS_ALLOWLIST:-.ban-globals-allowlist}"

# ripgrep is fast and consistent
if ! command -v rg >/dev/null 2>&1; then
  echo "ERROR: ripgrep (rg) is required." >&2
  exit 2
fi

# Patterns are conservative on purpose.
# If you truly need an exception, add an allowlist line with a justification.
PATTERNS=(
  '\bOnceLock\b'
  '\bLazyLock\b'
  '\blazy_static!\b'
  '\bonce_cell\b'
  '\bthread_local!\b'
  '\bstatic mut\b'
  '\binstall_[a-zA-Z0-9_]*\b'
)

echo "ban-globals: scanning paths:"
for p in $PATHS; do echo "  - $p"; done
echo

# Build rg args
RG_ARGS=(--hidden --no-ignore --glob '!.git/*' --glob '!target/*' --glob '!**/node_modules/*')

# Apply allowlist as inverted matches (each line is a regex or fixed substring)
# Allowlist format:
#   <pattern>\t<justification>
# or:
#   <pattern>
ALLOW_RG_EXCLUDES=()
if [[ -f "$ALLOWLIST" ]]; then
  # Read first column (pattern) per line, ignore comments
  while IFS= read -r line; do
    [[ -z "$line" ]] && continue
    [[ "$line" =~ ^# ]] && continue
    pat="${line%%$'\t'*}"
    pat="${pat%% *}"
    [[ -z "$pat" ]] && continue
    # Exclude lines matching allowlisted pattern
    ALLOW_RG_EXCLUDES+=(--glob "!$pat")
  done < "$ALLOWLIST"
fi

violations=0

for pat in "${PATTERNS[@]}"; do
  echo "Checking: $pat"
  # We can't "glob exclude by line"; allowlist is file-level. Keep it simple:
  # If you need surgical exceptions, prefer moving code or refactoring.
  if rg "${RG_ARGS[@]}" "${ALLOW_RG_EXCLUDES[@]}" -n -S "$pat" $PATHS; then
    echo
    violations=$((violations+1))
  else
    echo "  OK"
  fi
  echo
 done

if [[ $violations -ne 0 ]]; then
  echo "ban-globals: FAILED ($violations pattern group(s) matched)."
  echo "Fix the code or (rarely) justify an exception in $ALLOWLIST."
  exit 1
fi

echo "ban-globals: PASSED."
