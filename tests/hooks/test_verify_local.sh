#!/usr/bin/env bash
# SPDX-License-Identifier: Apache-2.0
# © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
set -euo pipefail

cd "$(dirname "${BASH_SOURCE[0]}")/../.." || exit 1

PASS=0
FAIL=0

pass() {
  echo "  PASS: $1"
  PASS=$((PASS + 1))
}

fail() {
  echo "  FAIL: $1"
  FAIL=$((FAIL + 1))
}

run_detect() {
  local tmp
  tmp="$(mktemp)"
  printf '%s\n' "$@" >"$tmp"
  VERIFY_CHANGED_FILES_FILE="$tmp" scripts/verify-local.sh detect
  rm -f "$tmp"
}

echo "=== verify-local classification ==="

docs_output="$(run_detect docs/plans/adr-0008-and-0009.md docs/ROADMAP/backlog/tooling-misc.md)"
if printf '%s\n' "$docs_output" | grep -q '^classification=docs$'; then
  pass "docs-only changes stay in docs mode"
else
  fail "docs-only changes should classify as docs"
  printf '%s\n' "$docs_output"
fi

reduced_output="$(run_detect crates/warp-cli/src/main.rs crates/echo-app-core/src/lib.rs)"
if printf '%s\n' "$reduced_output" | grep -q '^classification=reduced$'; then
  pass "non-critical crate changes use reduced mode"
else
  fail "non-critical crate changes should classify as reduced"
  printf '%s\n' "$reduced_output"
fi
if printf '%s\n' "$reduced_output" | grep -q '^changed_crates=echo-app-core,warp-cli$'; then
  pass "changed crate list is deduplicated and sorted"
else
  fail "changed crate list should be sorted and deduplicated"
  printf '%s\n' "$reduced_output"
fi

full_output="$(run_detect crates/warp-core/src/lib.rs)"
if printf '%s\n' "$full_output" | grep -q '^classification=full$'; then
  pass "warp-core changes force full verification"
else
  fail "warp-core changes should classify as full"
  printf '%s\n' "$full_output"
fi

workflow_output="$(run_detect .github/workflows/ci.yml)"
if printf '%s\n' "$workflow_output" | grep -q '^classification=full$'; then
  pass "workflow changes force full verification"
else
  fail "workflow changes should classify as full"
  printf '%s\n' "$workflow_output"
fi

echo "PASS: $PASS"
echo "FAIL: $FAIL"

if [[ $FAIL -gt 0 ]]; then
  exit 1
fi
