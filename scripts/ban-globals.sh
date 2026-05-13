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

# Patterns are conservative on purpose.
# If you truly need an exception, add an allowlist line with a justification.
PATTERNS=(
  '\bOnceLock\b'
  '\bLazyLock\b'
  '\blazy_static\!'
  '\bonce_cell\b'
  '\bthread_local\!'
  '\bstatic mut\b'
  '\binstall_[a-zA-Z0-9_]*\b'
)

echo "ban-globals: scanning paths:"
for p in $PATHS; do echo "  - $p"; done
echo

RG_ARGS=(--hidden --no-ignore --glob '!.git/*' --glob '!target/*' --glob '!**/node_modules/*')

# Apply allowlist as inverted matches (each line is a regex or fixed substring)
# Allowlist format:
#   <pattern>\t<justification>
# or:
#   <pattern>
ALLOW_RG_EXCLUDES=()
ALLOW_PATH_PATTERNS=()
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
    ALLOW_PATH_PATTERNS+=("$pat")
  done < "$ALLOWLIST"
fi

violations=0

is_allowlisted_path() {
  local file="$1"

  for pat in "${ALLOW_PATH_PATTERNS[@]}"; do
    if [[ "$file" == $pat ]]; then
      return 0
    fi
  done

  return 1
}

search_pattern_with_perl() {
  local pat="$1"
  local found=1
  local status=0

  if ! command -v perl >/dev/null 2>&1; then
    echo "ERROR: ripgrep (rg) or perl is required." >&2
    return 2
  fi

  while IFS= read -r -d '' file; do
    if is_allowlisted_path "$file"; then
      continue
    fi

    if SEARCH_PATTERN="$pat" perl -ne '
      BEGIN { $found = 0; $pattern = $ENV{"SEARCH_PATTERN"}; }
      if (/$pattern/) { print "$ARGV:$.:$_"; $found = 1; }
      END { exit($found ? 0 : 1); }
    ' "$file"; then
      found=0
    else
      status=$?
      if [[ $status -gt 1 ]]; then
        return "$status"
      fi
    fi
  done < <(find $PATHS -type f \
    \( -name '*.rs' \
      -o -name '*.toml' \
      -o -name '*.sh' \
      -o -name '*.mjs' \
      -o -name '*.js' \
      -o -name '*.ts' \
      -o -name '*.md' \
      -o -name '*.graphql' \
      -o -name '*.json' \
      -o -name '*.yaml' \
      -o -name '*.yml' \) \
    -not -path '*/.git/*' \
    -not -path '*/target/*' \
    -not -path '*/node_modules/*' \
    -print0)

  return "$found"
}

search_pattern() {
  local pat="$1"

  if command -v rg >/dev/null 2>&1; then
    rg "${RG_ARGS[@]}" "${ALLOW_RG_EXCLUDES[@]}" -n -S "$pat" $PATHS
    return $?
  fi

  search_pattern_with_perl "$pat"
}

for pat in "${PATTERNS[@]}"; do
  echo "Checking: $pat"
  # We can't "glob exclude by line"; allowlist is file-level. Keep it simple:
  # If you need surgical exceptions, prefer moving code or refactoring.
  if search_pattern "$pat"; then
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
