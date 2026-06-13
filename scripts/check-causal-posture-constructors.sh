#!/usr/bin/env bash
# SPDX-License-Identifier: Apache-2.0
# © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
#
# PLATFORM-0027 guard: posture-bearing records must be constructed through
# validated constructors, and CausalPosture must not regain a global default.

set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"

if [[ -n "${CAUSAL_POSTURE_LINT_PATHS:-}" ]]; then
  # shellcheck disable=SC2206
  SCAN_PATHS=(${CAUSAL_POSTURE_LINT_PATHS})
else
  SCAN_PATHS=("$ROOT_DIR/crates")
fi

violations=0

check_rg() {
  local label="$1"
  shift
  local output status

  set +e
  output="$(rg "$@" "${SCAN_PATHS[@]}" 2>&1)"
  status="$?"
  set -e

  if [[ "$status" -eq 0 ]]; then
    echo "causal-posture-constructors: ${label}" >&2
    echo "$output" >&2
    violations=1
    return
  fi

  if [[ "$status" -gt 1 ]]; then
    echo "$output" >&2
    exit "$status"
  fi
}

check_rg \
  "CausalPosture must not implement Default" \
  -n -P --glob '*.rs' \
  '\bimpl\s+Default\s+for\s+CausalPosture\b'

check_rg \
  "CausalPosture must not derive Default" \
  -n -P -U --glob '*.rs' \
  '#\s*\[derive\([^\]]*Default[^\]]*\)\]\s*pub\s+enum\s+CausalPosture\b'

check_rg \
  "CausalPosture::default() is forbidden; use named policy constructors" \
  -n -P --glob '*.rs' \
  '\bCausalPosture::default\s*\('

check_rg \
  "RetentionPosture and SessionContext literals bypass constructor invariants" \
  -n -P --glob '*.rs' \
  '(^|[=(:,]\s*)([A-Za-z0-9_]+::)*(RetentionPosture|SessionContext)\s*\{'

if [[ "$violations" -ne 0 ]]; then
  echo "causal-posture-constructors: FAILED." >&2
  echo "Use RetentionPosture::new and SessionContext::new so posture, authority, and admission scope are validated." >&2
  exit 1
fi

echo "causal-posture-constructors: PASSED."
