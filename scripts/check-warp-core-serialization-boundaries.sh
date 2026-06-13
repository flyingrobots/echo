#!/usr/bin/env bash
# SPDX-License-Identifier: Apache-2.0
# © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"

if ! command -v rg >/dev/null 2>&1; then
  echo "check-warp-core-serialization-boundaries: missing dependency: rg" >&2
  exit 1
fi

violations=0

report_violation() {
  local title="$1"
  local matches="$2"

  if [[ -z "$matches" ]]; then
    return
  fi

  echo "check-warp-core-serialization-boundaries: ${title}" >&2
  echo "$matches" >&2
  echo >&2
  violations=$((violations + 1))
}

manifest_matches="$(
  rg -n -S '^[[:space:]]*(serde|serde-value|ciborium)[[:space:]]*=' crates/warp-core/Cargo.toml || true
)"
report_violation \
  "warp-core manifest must not depend directly on serde/ciborium serialization crates" \
  "$manifest_matches"

feature_matches="$(
  rg -n -S '(^|[^[:alnum:]_])(dep:serde|bytes/serde|echo-runtime-schema/serde|warp-math/serde|"serde")' \
    crates/warp-core/Cargo.toml || true
)"
report_violation \
  "warp-core manifest must not expose serde feature plumbing" \
  "$feature_matches"

serde_code_matches="$(
  rg -n -S 'serde::|serde_json::|serde_wasm_bindgen::|cfg(_attr)?\(feature = "serde"|derive\([^)]*(Serialize|Deserialize)' \
    crates/warp-core/src crates/warp-core/tests \
    --glob '*.rs' || true
)"
report_violation \
  "warp-core source/tests must not use serde derives, serde cfgs, or serde serializers" \
  "$serde_code_matches"

boundary_call_matches="$(
  rg -n -S 'echo_wasm_abi::(encode_cbor|pack_[[:alnum:]_]+|unpack_[[:alnum:]_]+)|use echo_wasm_abi::\{[^}]*\b(encode_cbor|pack_[[:alnum:]_]+|unpack_[[:alnum:]_]+)' \
    crates/warp-core/src \
    --glob '*.rs' \
    --glob '!**/tests.rs' || true
)"

boundary_violations=""
if [[ -n "$boundary_call_matches" ]]; then
  while IFS= read -r match; do
    file="${match%%:*}"
    case "$file" in
      crates/warp-core/src/cmd.rs|\
      crates/warp-core/src/contract_host.rs|\
      crates/warp-core/src/coordinator.rs|\
      crates/warp-core/src/observation.rs|\
      crates/warp-core/src/optic.rs|\
      crates/warp-core/src/witnessed_suffix.rs)
        ;;
      *)
        boundary_violations+="${match}"$'\n'
        ;;
    esac
  done <<< "$boundary_call_matches"
fi
report_violation \
  "canonical ABI serialization calls must stay in explicit warp-core boundary modules" \
  "${boundary_violations%$'\n'}"

if [[ "$violations" -ne 0 ]]; then
  echo "check-warp-core-serialization-boundaries: FAILED (${violations} rule group(s) matched)." >&2
  exit 1
fi

echo "check-warp-core-serialization-boundaries: PASSED."
