#!/usr/bin/env bash
# SPDX-License-Identifier: Apache-2.0
# © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
set -euo pipefail

# Determinism Drill Sergeant: ban nondeterministic APIs and patterns
#
# Usage:
#   ./scripts/ban-nondeterminism.sh
#
# Optional env:
#   DETERMINISM_PATHS="crates/warp-core crates/warp-wasm crates/echo-wasm-abi"
#   DETERMINISM_ALLOWLIST=".ban-nondeterminism-allowlist"
#
# Allowlist governance: see docs/determinism/RELEASE_POLICY.md § "Determinism Allowlist Governance"
# for approval requirements, acceptable exemption criteria, and audit cadence.

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"

PATHS_DEFAULT="crates/warp-core crates/warp-wasm crates/echo-wasm-abi"
PATHS="${DETERMINISM_PATHS:-$PATHS_DEFAULT}"

ALLOWLIST="${DETERMINISM_ALLOWLIST:-.ban-nondeterminism-allowlist}"

RG_ARGS=(
  --hidden
  --no-ignore
  --glob '!**/.git/**'
  --glob '!**/target/**'
  --glob '!**/node_modules/**'
  --glob '!**/.clippy.toml'
)

# You can allow file-level exceptions via allowlist (keep it tiny).
ALLOW_GLOBS=()
ALLOW_PATH_PATTERNS=()
if [[ -f "$ALLOWLIST" ]]; then
  while IFS= read -r line; do
    [[ -z "$line" ]] && continue
    [[ "$line" =~ ^# ]] && continue
    pat="${line%%$'\t'*}"
    pat="${pat%% *}"
    [[ -z "$pat" ]] && continue
    ALLOW_GLOBS+=(--glob "!$pat")
    ALLOW_PATH_PATTERNS+=("$pat")
  done < "$ALLOWLIST"
fi

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
    -not -name '.clippy.toml' \
    -print0)

  return "$found"
}

search_pattern() {
  local pat="$1"

  if command -v rg >/dev/null 2>&1; then
    rg "${RG_ARGS[@]}" "${ALLOW_GLOBS[@]}" -n -S "$pat" $PATHS
    return $?
  fi

  search_pattern_with_perl "$pat"
}

# Patterns: conservative and intentionally annoying.
# If you hit a false positive, refactor; don't immediately allowlist.
PATTERNS=(
  # Time / entropy (core determinism killers)
  '\bstd::time::SystemTime\b'
  '\bSystemTime::now\b'
  '\bstd::time::Instant\b'
  '\bInstant::now\b'
  '\bstd::thread::sleep\b'
  '\b(tokio|async_std)::time::sleep\b'

  # Randomness
  '\brand::\b'
  '\bgetrandom::\b'
  '\bfastrand::\b'

  # Host-supplied callback / network escape hatches
  '\bjs_sys::Function\b'
  '\bwasm_bindgen::closure::Closure\b'
  '\bClosure<'
  '\bstd::net::\b'
  '\breqwest::\b'
  '\bureq::\b'

  # Unordered containers that will betray you if they cross a boundary
  '\bstd::collections::HashMap\b'
  '\bstd::collections::HashSet\b'
  '\bhashbrown::HashMap\b'
  '\bhashbrown::HashSet\b'

  # JSON & “helpful” serialization in core paths
  '\bserde_json::\b'
  '\bserde_wasm_bindgen::\b'

  # Float nondeterminism hotspots (you can tune these)
  '\b(f32|f64)::NAN\b'
  '\b(f32|f64)::INFINITY\b'
  '\b(f32|f64)::NEG_INFINITY\b'
  '\.sin\('
  '\.cos\('
  '\.tan\('
  '\.sqrt\('
  '\.pow[f]?\('

  # Host/environment variability
  '\bstd::env::\b'
  '\bstd::fs::\b'
  '\bstd::process::\b'

  # Concurrency primitives (optional—uncomment if you want core to be single-thread-only)
  # '\bstd::sync::Mutex\b'
  # '\bstd::sync::RwLock\b'
  # '\bstd::sync::atomic::\b'
)

echo "ban-nondeterminism: scanning paths:"
for p in $PATHS; do echo "  - $p"; done
echo

violations=0
for pat in "${PATTERNS[@]}"; do
  echo "Checking: $pat"
  if search_pattern "$pat"; then
    echo
    violations=$((violations+1))
  else
    echo "  OK"
  fi
  echo
 done

if [[ $violations -ne 0 ]]; then
  echo "ban-nondeterminism: FAILED ($violations pattern group(s) matched)."
  echo "Fix the code or (rarely) justify an exception in $ALLOWLIST."
  exit 1
fi

echo "ban-nondeterminism: PASSED."
