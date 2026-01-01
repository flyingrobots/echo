#!/usr/bin/env bash
# SPDX-License-Identifier: Apache-2.0
# © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>

set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"

cd "$ROOT"

if ! command -v rg >/dev/null 2>&1; then
  echo "Error: ripgrep (rg) is required for deterministic math guard checks." >&2
  exit 1
fi

# Policy: runtime math modules must not call platform/libm transcendentals
# directly. Trig should flow through `warp_core::math::trig` and be surfaced via
# `F32Scalar` (or future fixed-point scalar types).
#
# We enforce this narrowly within `warp-core`'s math module, excluding:
# - scalar.rs: the sanctioned wrapper surface
# - trig.rs / trig_lut.rs: the deterministic backend + data
target_dir="crates/warp-core/src/math"

matches="$(
  rg -n --no-heading --color never '\.(sin|cos|sin_cos)\(' "$target_dir" \
    --glob '!scalar.rs' \
    --glob '!trig.rs' \
    --glob '!trig_lut.rs' \
    || true
)"

if [[ -n "$matches" ]]; then
  echo "Error: raw trig calls found in warp-core math module (use math::trig or F32Scalar wrappers):" >&2
  echo "$matches" >&2
  exit 1
fi

echo "ok: no raw trig calls found in $target_dir"

