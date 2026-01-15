#!/usr/bin/env bash
set -euo pipefail

# Determinism Drill Sergeant: ban nondeterministic APIs and patterns
#
# Usage:
#   ./scripts/ban-nondeterminism.sh
#
# Optional env:
#   DETERMINISM_PATHS="crates/warp-core crates/warp-wasm crates/echo-wasm-abi"
#   DETERMINISM_ALLOWLIST=".ban-nondeterminism-allowlist"

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"

if ! command -v rg >/dev/null 2>&1; then
  echo "ERROR: ripgrep (rg) is required." >&2
  exit 2
fi

PATHS_DEFAULT="crates/warp-core crates/warp-wasm crates/echo-wasm-abi"
PATHS="${DETERMINISM_PATHS:-$PATHS_DEFAULT}"

ALLOWLIST="${DETERMINISM_ALLOWLIST:-.ban-nondeterminism-allowlist}"

RG_ARGS=(--hidden --no-ignore --glob '!.git/*' --glob '!target/*' --glob '!**/node_modules/*')

# You can allow file-level exceptions via allowlist (keep it tiny).
ALLOW_GLOBS=()
if [[ -f "$ALLOWLIST" ]]; then
  while IFS= read -r line; do
    [[ -z "$line" ]] && continue
    [[ "$line" =~ ^# ]] && continue
    pat="${line%%$'\t'*}"
    pat="${pat%% *}"
    [[ -z "$pat" ]] && continue
    ALLOW_GLOBS+=(--glob "!$pat")
  done < "$ALLOWLIST"
fi

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
  if rg "${RG_ARGS[@]}" "${ALLOW_GLOBS[@]}" -n -S "$pat" $PATHS; then
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
