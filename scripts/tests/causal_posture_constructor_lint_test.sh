#!/usr/bin/env bash
# SPDX-License-Identifier: Apache-2.0
# © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
#
# Regression coverage for PLATFORM-0027 constructor posture lint.

set -euo pipefail

script_root="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
repo_root="$(cd -- "${script_root}/../.." && pwd)"
guard="${repo_root}/scripts/check-causal-posture-constructors.sh"

passed=0
failed=0

assert() {
  local label="$1"
  shift
  if (set +e; "$@" >/dev/null 2>&1); then
    echo "  PASS: ${label}"
    ((passed++)) || true
  else
    echo "  FAIL: ${label}"
    ((failed++)) || true
  fi
}

assert_not() {
  local label="$1"
  shift
  if (set +e; "$@" >/dev/null 2>&1); then
    echo "  FAIL: ${label}"
    ((failed++)) || true
  else
    echo "  PASS: ${label}"
    ((passed++)) || true
  fi
}

echo "=== Causal posture constructor lint tests ==="
echo ""

echo "1. Guard exists"
assert "constructor posture guard exists" \
  test -x "${guard}"

echo ""
echo "2. Raw retained posture construction is rejected"
tmpdir="$(mktemp -d "${TMPDIR:-/tmp}/causal-posture-lint.XXXXXX")"
trap 'rm -rf "${tmpdir}"' EXIT
fixture="${tmpdir}/bad.rs"
cat >"${fixture}" <<'RS'
fn bad_fixture() {
    let _posture = RetentionPosture {
        causal_posture,
        posture_derivation,
        authority,
        retention_contract,
        admission_scope,
    };
}
RS

assert_not "raw RetentionPosture literal is rejected" \
  env CAUSAL_POSTURE_LINT_PATHS="${fixture}" "${guard}"

echo ""
echo "=== Results: ${passed} passed, ${failed} failed ==="

if [ "${failed}" -gt 0 ]; then
  exit 1
fi
