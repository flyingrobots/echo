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
#   DETERMINISM_PATHS="crates/warp-core crates/warp-math crates/warp-wasm crates/echo-wasm-abi crates/echo-edict-canonical crates/echo-edict-provider-lowerer crates/echo-edict-provider-verifier"
#   DETERMINISM_ALLOWLIST=".ban-nondeterminism-allowlist"
#
# Every waiver is rule-scoped to an exact path and must explain why the
# nondeterministic API cannot influence semantic history.

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"
source scripts/lib/determinism-scan.sh

PATHS_DEFAULT="crates/warp-core crates/warp-math crates/warp-wasm crates/echo-wasm-abi crates/echo-edict-canonical crates/echo-edict-provider-lowerer crates/echo-edict-provider-verifier"
PATHS="${DETERMINISM_PATHS:-$PATHS_DEFAULT}"

ALLOWLIST="${DETERMINISM_ALLOWLIST:-.ban-nondeterminism-allowlist}"
det_load_waivers "$ALLOWLIST"

# Patterns: conservative and intentionally annoying.
# If you hit a false positive, refactor; don't immediately allowlist.
PATTERNS=(
  # Time / entropy (core determinism killers)
  'time-system	\bstd::time::SystemTime\b'
  'time-system	\bSystemTime::now\b'
  'time-instant	\bstd::time::Instant\b'
  'time-instant	\bInstant::now\b'
  'thread-sleep	\bstd::thread::sleep\b'
  'thread-sleep	\b(tokio|async_std)::time::sleep\b'
  'host-parallelism	\bstd::thread::available_parallelism\b'
  'host-parallelism	\bavailable_parallelism[[:space:]]*\('

  # Randomness
  'random	\brand::\b'
  'random	\bgetrandom::\b'
  'random	\bfastrand::\b'

  # Host-supplied callback / network escape hatches
  'host-callback	\bjs_sys::Function\b'
  'host-callback	\bwasm_bindgen::closure::Closure\b'
  'host-callback	\bClosure<'
  'network	\bstd::net::\b'
  'network	\breqwest::\b'
  'network	\bureq::\b'

  # Unordered containers that will betray you if they cross a boundary
  'unordered-container	\bstd::collections::HashMap\b'
  'unordered-container	\bstd::collections::HashSet\b'
  'unordered-container	\bhashbrown::HashMap\b'
  'unordered-container	\bhashbrown::HashSet\b'
  'unordered-container	\brustc_hash::FxHashMap\b'
  'unordered-container	\brustc_hash::FxHashSet\b'
  'unordered-container	\buse[[:space:]]+std::collections::\{[^;]*\bHash(Map|Set)\b'
  'unordered-container	\buse[[:space:]]+rustc_hash::\{?[^;]*\bFxHash(Map|Set)\b'
  'unordered-container	\b(HashMap|HashSet|FxHashMap|FxHashSet)[[:space:]]*<'

  # JSON & “helpful” serialization in core paths
  'json-serialization	\bserde_json::\b'
  'wasm-serde	\bserde_wasm_bindgen::\b'

  # Float nondeterminism hotspots (you can tune these)
  'float-sentinel	\b(f32|f64)::NAN\b'
  'float-sentinel	\b(f32|f64)::INFINITY\b'
  'float-sentinel	\b(f32|f64)::NEG_INFINITY\b'
  'float-op	\.sin\('
  'float-op	\.cos\('
  'float-op	\.tan\('
  'float-op	\.sqrt\('
  'float-op	\.pow[f]?\('

  # Host/environment variability
  'std-env	\bstd::env(::|[[:space:]]*[;,{])|\buse[[:space:]]+std::\{[^;]*\benv\b|\benv::(args|args_os|current_dir|current_exe|home_dir|join_paths|remove_var|set_current_dir|set_var|split_paths|temp_dir|var|var_os|vars|vars_os)\b'
  'std-fs	\bstd::fs(::|[[:space:]]*[;,{])|\buse[[:space:]]+std::\{[^;]*\bfs\b|\bfs::(canonicalize|copy|create_dir|create_dir_all|hard_link|metadata|read|read_dir|read_link|read_to_string|remove_dir|remove_dir_all|remove_file|rename|set_permissions|soft_link|symlink_metadata|write)\b'
  'std-process	\bstd::process(::|[[:space:]]*[;,{])|\buse[[:space:]]+std::\{[^;]*\bprocess\b|\bprocess::(abort|Command|exit|id)\b'

  # Concurrency primitives (optional—uncomment if you want core to be single-thread-only)
  # '\bstd::sync::Mutex\b'
  # '\bstd::sync::RwLock\b'
  # '\bstd::sync::atomic::\b'
)

echo "ban-nondeterminism: scanning paths:"
for p in $PATHS; do echo "  - $p"; done
echo

violations=0
for entry in "${PATTERNS[@]}"; do
  rule="${entry%%$'\t'*}"
  pat="${entry#*$'\t'}"
  echo "Checking [$rule]: $pat"
  if det_scan_line "$rule" "$pat" $PATHS; then
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

serialization_guard="scripts/check-warp-core-serialization-boundaries.sh"
if [[ ! -f "$serialization_guard" ]]; then
  echo "ban-nondeterminism: missing required guard: $serialization_guard" >&2
  exit 1
fi
bash "$serialization_guard"

echo "ban-nondeterminism: PASSED."
