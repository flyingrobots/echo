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
#   - any static whose declared type does not begin, on the same line, with a
#     shared reference. That admits `static T: &[u8]` and refuses a Mutex, an
#     atomic, a cell, an array (which can hold atomics), a struct that could
#     hold one, and a type wrapped onto the next line. The rule fails closed:
#     it does not try to recognise which types are mutable.
#   - a reference-typed static that names a mutable type on its line
#   - a call to `module::install_*(...)`: an installer reached through a
#     module path, whose backing static may live outside the scanned crates
#
# Known limits of a line scanner: a reference-typed static whose mutable type
# wraps onto the next line, and an installer imported by `use` and then
# called by its bare name. Methods and helpers named install_* that take
# their state as an argument are not global and are not matched.
#
# Usage:
#   ./scripts/ban-globals.sh
#
# Optional env:
#   BAN_GLOBALS_PATHS="crates/warp-core crates/warp-math crates/warp-wasm crates/echo-wasm-abi crates/echo-edict-canonical crates/echo-edict-provider-lowerer crates/echo-edict-provider-verifier"
#   BAN_GLOBALS_ALLOWLIST=".ban-globals-allowlist"
#   DETERMINISM_FORCE_NO_RG=1   use the perl/find backend even when rg exists
#
# Every waiver is rule-scoped to an exact path and must explain why the
# global cannot carry hidden state between runs.

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"
source scripts/lib/determinism-scan.sh

PATHS_DEFAULT="crates/warp-core crates/warp-math crates/warp-wasm crates/echo-wasm-abi crates/echo-edict-canonical crates/echo-edict-provider-lowerer crates/echo-edict-provider-verifier"
PATHS="${BAN_GLOBALS_PATHS:-$PATHS_DEFAULT}"

ALLOWLIST="${BAN_GLOBALS_ALLOWLIST:-.ban-globals-allowlist}"
det_load_waivers "$ALLOWLIST"

# Patterns are conservative on purpose.
# If you truly need an exception, add a rule-scoped waiver with a justification.
PATTERNS=(
  'once-lock	\bOnceLock\b'
  'lazy-lock	\bLazyLock\b'
  'lazy-static	\blazy_static!'
  'once-cell	\bonce_cell\b'
  'thread-local	\bthread_local!'
  'static-mut	\bstatic[[:space:]]+mut\b'
  'static-non-reference	\bstatic[[:space:]]+[A-Za-z_][A-Za-z0-9_]*[[:space:]]*:[[:space:]]*([^&[:space:]]|$)'
  'static-interior-reference	\bstatic[[:space:]]+[A-Za-z_][A-Za-z0-9_]*[[:space:]]*:[[:space:]]*&.*\b(Mutex|RwLock|Atomic[A-Za-z0-9]*|Cell|RefCell|UnsafeCell|OnceLock|LazyLock|OnceCell)\b'
  'external-install	(^|[^A-Za-z0-9_])[a-z_][a-z0-9_]*::install_[A-Za-z0-9_]*[[:space:]]*\('
)

echo "ban-globals: scanning paths:"
for p in $PATHS; do echo "  - $p"; done
echo

violations=0
for entry in "${PATTERNS[@]}"; do
  rule="${entry%%$'\t'*}"
  pat="${entry#*$'\t'}"
  echo "Checking [$rule]: $pat"
  status=0
  det_scan_line "$rule" "$pat" $PATHS || status=$?
  if [[ $status -eq 0 ]]; then
    echo
    violations=$((violations+1))
  elif [[ $status -eq 1 ]]; then
    echo "  OK"
  else
    echo "ban-globals: scanner failed for [$rule] (status $status)." >&2
    exit "$status"
  fi
  echo
done

if [[ $violations -ne 0 ]]; then
  echo "ban-globals: FAILED ($violations pattern group(s) matched)."
  echo "Fix the code or (rarely) justify an exception in $ALLOWLIST."
  exit 1
fi

echo "ban-globals: PASSED."
