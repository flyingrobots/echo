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
warp_core_lib="${repo_root}/crates/warp-core/src/lib.rs"

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
echo "4. Default public API does not export native rule authoring"
assert "default lib.rs does not unconditionally pub use RewriteRule" \
  awk '
    /^#\[cfg\(feature = "native_rule_bootstrap"\)\]$/ { gated = 1; next }
    gated && (/^[[:space:]]*$/ || /^[[:space:]]*\/\//) { next }
    /^pub use rule::\{ConflictPolicy, ExecuteFn, MatchFn, PatternGraph, RewriteRule\};$/ {
      if (!gated) exit 1
    }
    { gated = 0 }
    END { exit 0 }
  ' "${warp_core_lib}"

tmp_gated_export="$(mktemp)"
cat >"${tmp_gated_export}" <<'EOF'
#[cfg(feature = "native_rule_bootstrap")]

// bootstrap export stays gated even when separated by a comment
pub use rule::{ConflictPolicy, ExecuteFn, MatchFn, PatternGraph, RewriteRule};
EOF
assert "native bootstrap cfg gate survives blank/comment separation" \
  awk '
    /^#\[cfg\(feature = "native_rule_bootstrap"\)\]$/ { gated = 1; next }
    gated && (/^[[:space:]]*$/ || /^[[:space:]]*\/\//) { next }
    /^pub use rule::\{ConflictPolicy, ExecuteFn, MatchFn, PatternGraph, RewriteRule\};$/ {
      if (!gated) exit 1
    }
    { gated = 0 }
    END { exit 0 }
  ' "${tmp_gated_export}"
rm -f "${tmp_gated_export}"

echo ""
echo "=== Results: ${passed} passed, ${failed} failed ==="

if [ "${failed}" -gt 0 ]; then
  exit 1
fi
