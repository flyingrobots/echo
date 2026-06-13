#!/usr/bin/env bash
# SPDX-License-Identifier: Apache-2.0
# © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"
source scripts/lib/determinism-scan.sh
det_load_waivers /dev/null

violations=0
ABI_SERIALIZERS='(encode_cbor|pack_intent_v1|unpack_intent_v1|pack_control_intent_v1|unpack_control_intent_v1|pack_import_suffix_intent_v1|unpack_import_suffix_intent_v1)'

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
  det_scan_line "warp-core-serialization-manifest" \
    '^[[:space:]]*(serde|serde-value|ciborium)[[:space:]]*=' \
    crates/warp-core/Cargo.toml || true
  det_scan_line "warp-core-serialization-manifest" \
    '^[[:space:]]*(serde|serde-value|ciborium)\.[[:alnum:]_-]+[[:space:]]*=' \
    crates/warp-core/Cargo.toml || true
  det_scan_line "warp-core-serialization-manifest" \
    '^[[:space:]]*\[(dependencies|dev-dependencies|build-dependencies)\.(serde|serde-value|ciborium)\]' \
    crates/warp-core/Cargo.toml || true
)"
report_violation \
  "warp-core manifest must not depend directly on serde/ciborium serialization crates" \
  "$manifest_matches"

feature_matches="$(
  det_scan_line "warp-core-serialization-feature" \
    '(^|[^[:alnum:]_])(dep:serde|bytes/serde|echo-runtime-schema/serde|warp-math/serde|"serde")' \
    crates/warp-core/Cargo.toml || true
)"
report_violation \
  "warp-core manifest must not expose serde feature plumbing" \
  "$feature_matches"

serde_code_matches="$(
  det_scan_line "warp-core-serde-code" \
    'serde::|serde_json::|serde_wasm_bindgen::|cfg(_attr)?\(feature = "serde"|derive\([^)]*(Serialize|Deserialize)' \
    crates/warp-core/src crates/warp-core/tests || true
)"
report_violation \
  "warp-core source/tests must not use serde derives, serde cfgs, or serde serializers" \
  "$serde_code_matches"

boundary_call_matches="$(
  det_scan_line "warp-core-abi-serialization-boundary" \
    "echo_wasm_abi::${ABI_SERIALIZERS}\\b" \
    crates/warp-core/src || true
  det_scan_line "warp-core-abi-serialization-boundary" \
    "\\buse[[:space:]]+echo_wasm_abi::[^;]*${ABI_SERIALIZERS}\\b" \
    crates/warp-core/src || true
  det_scan_line "warp-core-abi-serialization-boundary" \
    "\\b${ABI_SERIALIZERS}[[:space:]]*[,;(]" \
    crates/warp-core/src || true
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
      crates/warp-core/src/witnessed_suffix.rs|\
      */tests.rs|\
      *_tests.rs)
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
