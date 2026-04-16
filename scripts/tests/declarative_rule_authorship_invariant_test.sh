#!/usr/bin/env bash
# SPDX-License-Identifier: Apache-2.0
# © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
#
# Tests for cycle 0012: DECLARATIVE-RULE-AUTHORSHIP invariant document.

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

invariant="${repo_root}/docs/invariants/DECLARATIVE-RULE-AUTHORSHIP.md"
release_policy="${repo_root}/docs/RELEASE_POLICY.md"

echo "=== DECLARATIVE-RULE-AUTHORSHIP invariant tests ==="
echo ""

echo "1. Invariant document exists"
assert "docs/invariants/DECLARATIVE-RULE-AUTHORSHIP.md exists" \
  test -f "${invariant}"

echo ""
echo "2. Normative language"
assert "contains MUST" \
  grep -q "MUST" "${invariant}"
assert "contains Wesley-compiled declarative IR" \
  grep -qi "Wesley-compiled declarative IR" "${invariant}"
assert "contains bootstrap-only wording" \
  grep -qi "bootstrap-only" "${invariant}"
assert "contains callback-free wording" \
  grep -qi "callback-free" "${invariant}"

echo ""
echo "3. Release policy cross-reference"
assert "RELEASE_POLICY references DECLARATIVE-RULE-AUTHORSHIP" \
  grep -q "DECLARATIVE-RULE-AUTHORSHIP" "${release_policy}"

echo ""
echo "=== Results: ${passed} passed, ${failed} failed ==="

if [ "${failed}" -gt 0 ]; then
  exit 1
fi
