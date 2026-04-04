#!/usr/bin/env bash
# SPDX-License-Identifier: Apache-2.0
# © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
#
# Tests for cycle 0004: STRAND-CONTRACT invariant document.

set -euo pipefail

script_root="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
repo_root="$(cd -- "${script_root}/../.." && pwd)"

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

invariant="${repo_root}/docs/invariants/STRAND-CONTRACT.md"

echo "=== STRAND-CONTRACT invariant tests ==="
echo ""

echo "1. Invariant document exists"
assert "docs/invariants/STRAND-CONTRACT.md exists" \
  test -f "${invariant}"

echo ""
echo "2. Contains all ten invariant codes"
for code in INV-S1 INV-S2 INV-S3 INV-S4 INV-S5 INV-S6 INV-S7 INV-S8 INV-S9 INV-S10; do
  assert "${code} present" \
    grep -q "${code}" "${invariant}"
done

echo ""
echo "3. Normative language"
assert "contains MUST" \
  grep -q "MUST" "${invariant}"

echo ""
echo "=== Results: ${passed} passed, ${failed} failed ==="

if [ "${failed}" -gt 0 ]; then
  exit 1
fi
