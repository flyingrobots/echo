#!/usr/bin/env bash
# SPDX-License-Identifier: Apache-2.0
# © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>

set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"

cd "$ROOT"

# Policy: runtime math modules must not call platform/libm transcendentals
# directly. Trig should flow through `warp_core::math::trig` and be surfaced via
# `F32Scalar` (or future fixed-point scalar types).
#
# We enforce this narrowly within `warp-core`'s math module, excluding:
# - scalar.rs: the sanctioned wrapper surface
# - trig.rs / trig_lut.rs: the deterministic backend + data
target_dir="crates/warp-core/src/math"
# Match method calls like `.sin(`, allowing optional whitespace before the `(`.
# Use explicit character classes to keep the regex compatible across `rg` and `grep`.
pattern='[.](sin|cos|sin_cos)[[:space:]]*[(]'

if [[ ! -d "$target_dir" ]]; then
  echo "Error: determinism guard target directory not found: $target_dir" >&2
  echo "If the warp-core math module moved, update scripts/check_no_raw_trig.sh accordingly." >&2
  exit 1
fi

if ! git rev-parse --is-inside-work-tree >/dev/null 2>&1; then
  echo "Error: this script must run inside a git work tree (for deterministic file enumeration)." >&2
  exit 1
fi

files=()
while IFS= read -r -d '' path; do
  case "$path" in
    *.rs)
      base="${path##*/}"
      if [[ "$base" == "scalar.rs" || "$base" == "trig.rs" || "$base" == "trig_lut.rs" ]]; then
        continue
      fi
      files+=("$path")
      ;;
  esac
done < <(git ls-files -z -- "$target_dir")

if [[ ${#files[@]} -eq 0 ]]; then
  echo "Error: no Rust source files found under $target_dir (did paths change?)" >&2
  exit 1
fi

if command -v rg >/dev/null 2>&1; then
  set +e
  matches="$(rg -n --no-heading --color never "$pattern" -- "${files[@]}")"
  status=$?
  set -e
  if [[ $status -gt 1 ]]; then
    echo "$matches" >&2
    exit $status
  fi
else
  # CI runners may not have ripgrep installed by default; fall back to `grep`.
  # Both lanes use the same `git ls-files` input set to avoid drift.
  set +e
  matches="$(grep -nE "$pattern" -- "${files[@]}")"
  status=$?
  set -e
  if [[ $status -gt 1 ]]; then
    echo "$matches" >&2
    exit $status
  fi
fi

if [[ -n "$matches" ]]; then
  echo "Error: raw trig calls found in warp-core math module (use math::trig or F32Scalar wrappers):" >&2
  echo "$matches" >&2
  exit 1
fi

tool="grep"
if command -v rg >/dev/null 2>&1; then
  tool="rg"
fi
echo "ok: no raw trig calls found in $target_dir (scanned ${#files[@]} files; tool=$tool)"
