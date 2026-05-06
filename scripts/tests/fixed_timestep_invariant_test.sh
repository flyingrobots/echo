#!/usr/bin/env bash
# SPDX-License-Identifier: Apache-2.0
# © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
#
# Tests for cycle 0003: FIXED-TIMESTEP invariant document.

set -euo pipefail

script_root="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
repo_root="$(cd -- "${script_root}/../.." && pwd)"

fail() {
  echo "FAIL: $*" >&2
  exit 1
}

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

invariant="${repo_root}/docs/invariants/FIXED-TIMESTEP.md"
spec004="${repo_root}/docs/spec/SPEC-0004-worldlines-playback-truthbus.md"

echo "=== FIXED-TIMESTEP invariant tests ==="
echo ""

# --- Existence ---
echo "1. Invariant document exists"
assert "docs/invariants/FIXED-TIMESTEP.md exists" \
  test -f "${invariant}"

# --- Normative language ---
echo ""
echo "2. Normative language"
assert "contains MUST" \
  grep -q "MUST" "${invariant}"
assert "contains tick_quantum" \
  grep -q "tick_quantum" "${invariant}"

# --- Seven rulings ---
echo ""
echo "3. Seven normative rulings"
assert "ruling 1: immutable tick_quantum at genesis" \
  grep -qi "immutable.*tick_quantum" "${invariant}"
assert "ruling 2: each tick advances by exactly one quantum" \
  grep -qi "exactly one" "${invariant}"
assert "ruling 3: dt is not an admitted stream fact" \
  grep -qi "not an admitted stream fact" "${invariant}"
assert "ruling 4: dt is never stored per tick" \
  grep -qi "never stored per tick" "${invariant}"
assert "ruling 5: tick-denominated" \
  grep -qi "tick-denominated" "${invariant}"
assert "ruling 6: canonical decision" \
  grep -qi "canonical decision" "${invariant}"
assert "ruling 7: identical tick_quantum for cross-worldline" \
  grep -qi "identical.*tick_quantum" "${invariant}"

# --- Cross-references ---
echo ""
echo "4. HistoryTime / HostTime classification"
assert "classifies HistoryTime" \
  grep -q "HistoryTime" "${invariant}"
assert "classifies HostTime" \
  grep -q "HostTime" "${invariant}"
assert "legacy OpEnvelope timestamp is HostTime" \
  grep -q "Legacy \`OpEnvelope.ts\`.*HostTime" "${invariant}"
assert "deadlineTick is HistoryTime" \
  grep -q "deadlineTick.*HistoryTime" "${invariant}"

# --- Cross-references ---
echo ""
echo "5. Cross-references"
assert "SPEC-0004 references the invariant" \
  grep -qi "FIXED-TIMESTEP" "${spec004}"
assert "static nondeterminism guard is referenced" \
  grep -q "scripts/ban-nondeterminism.sh" "${invariant}"
assert "release allowlist policy is referenced" \
  grep -q "docs/determinism/RELEASE_POLICY.md" "${invariant}"

# --- Static wall-clock guard ---
echo ""
echo "6. Static wall-clock guard"
guard="${repo_root}/scripts/ban-nondeterminism.sh"
assert "ban-nondeterminism guard exists" \
  test -x "${guard}"
assert "ban-nondeterminism bans SystemTime" \
  grep -q "SystemTime" "${guard}"
assert "ban-nondeterminism bans Instant" \
  grep -q "Instant" "${guard}"

# --- Negative test: no variable-dt concepts in crates ---
echo ""
echo "7. Negative test: variable-dt concepts absent from crates"
assert_not "no 'variable_dt' in crates/" \
  grep -r "variable_dt" "${repo_root}/crates/"
assert_not "no 'dt_stream' in crates/" \
  grep -r "dt_stream" "${repo_root}/crates/"
assert_not "no 'variable.dt' in crates/ (dot-separated)" \
  grep -rP "variable\.dt" "${repo_root}/crates/"

echo ""
echo "=== Results: ${passed} passed, ${failed} failed ==="

if [ "${failed}" -gt 0 ]; then
  exit 1
fi
