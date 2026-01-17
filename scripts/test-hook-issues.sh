#!/usr/bin/env bash
# SPDX-License-Identifier: Apache-2.0
# © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
# Test script to verify code review issues in git hooks
set -euo pipefail

PASS=0
FAIL=0

pass() {
  echo "✅ PASS: $1"
  PASS=$((PASS + 1))
}

fail() {
  echo "❌ FAIL: $1"
  FAIL=$((FAIL + 1))
}

echo "=== Testing Code Review Issues ==="
echo ""

# Issue 1: pre-push-parallel uses GNU-only xargs -r (line 38)
echo "--- Test 1: xargs -r portability in pre-push-parallel ---"
# Only match actual code, not comments (lines starting with #)
if grep -v '^\s*#' .githooks/pre-push-parallel | grep -q 'xargs -r'; then
  fail "pre-push-parallel uses GNU-only 'xargs -r' (not portable to macOS/BSD)"
  grep -n 'xargs -r' .githooks/pre-push-parallel | grep -v '^\s*#'
else
  pass "pre-push-parallel does not use 'xargs -r' in actual code"
fi
echo ""

# Issue 2: pre-push-parallel set -e aborts on first wait failure
echo "--- Test 2: set -e during wait collection in pre-push-parallel ---"
# Check if there's a set +e before the wait block
if grep -A5 '# Wait and collect results' .githooks/pre-push-parallel | grep -q 'set +e'; then
  pass "pre-push-parallel disables errexit before wait block"
else
  fail "pre-push-parallel has set -e active during wait block (first failure aborts before collecting all results)"
  echo "  The wait block needs 'set +e' before and 'set -e' after to collect all exit codes"
fi
echo ""

# Issue 3: pre-push-sequential runs rg twice for each pattern
echo "--- Test 3: Double rg invocations in pre-push-sequential ---"
# Count actual rg invocations (lines starting with 'rg' or containing '$(rg'), not comments/echo
MISSING_DOCS_RG_COUNT=$(grep -E '^\s*(rg|if match_output=\$\(rg)' .githooks/pre-push-sequential | grep -c "missing_docs" || echo "0")
if [[ "$MISSING_DOCS_RG_COUNT" -gt 1 ]]; then
  fail "pre-push-sequential runs rg twice for missing_docs pattern (found $MISSING_DOCS_RG_COUNT invocations)"
  grep -En '^\s*(rg|if match_output=\$\(rg)' .githooks/pre-push-sequential | grep "missing_docs"
else
  pass "pre-push-sequential runs rg only once for missing_docs pattern"
fi

# Count actual rg invocations for no_mangle pattern
NO_MANGLE_RG_COUNT=$(grep -E '^\s*(rg|if match_output=\$\(rg)' .githooks/pre-push-sequential | grep -c "no_mangle" || echo "0")
if [[ "$NO_MANGLE_RG_COUNT" -gt 1 ]]; then
  fail "pre-push-sequential runs rg twice for no_mangle pattern (found $NO_MANGLE_RG_COUNT invocations)"
  grep -En '^\s*(rg|if match_output=\$\(rg)' .githooks/pre-push-sequential | grep "no_mangle"
else
  pass "pre-push-sequential runs rg only once for no_mangle pattern"
fi
echo ""

# ============================================
# Verify issues marked as "already fixed"
# ============================================
echo "=== Verifying Previously Fixed Issues ==="
echo ""

# Verify: pre-push-parallel uses +$PINNED for fmt/clippy/tests
echo "--- Verify: Toolchain pinning in pre-push-parallel ---"
if grep -E 'cargo \+"\$PINNED"' .githooks/pre-push-parallel | grep -q 'fmt'; then
  pass "run_fmt uses +\$PINNED"
else
  fail "run_fmt does NOT use +\$PINNED"
fi
if grep -E 'cargo \+"\$PINNED"' .githooks/pre-push-parallel | grep -q 'clippy'; then
  pass "run_clippy uses +\$PINNED"
else
  fail "run_clippy does NOT use +\$PINNED"
fi
if grep -E 'cargo \+"\$PINNED"' .githooks/pre-push-parallel | grep -qE '(nextest|test)'; then
  pass "run_tests uses +\$PINNED"
else
  fail "run_tests does NOT use +\$PINNED"
fi
echo ""

# Verify: pre-push-parallel has toolchain check
echo "--- Verify: Toolchain check in pre-push-parallel ---"
if grep -q 'rustup toolchain list' .githooks/pre-push-parallel && grep -q 'missing toolchain' .githooks/pre-push-parallel; then
  pass "pre-push-parallel checks if pinned toolchain is installed"
else
  fail "pre-push-parallel does NOT check if pinned toolchain is installed"
fi
echo ""

# Verify: pre-push-parallel has set -e
echo "--- Verify: set -e in pre-push-parallel ---"
if head -10 .githooks/pre-push-parallel | grep -q 'set -euo pipefail'; then
  pass "pre-push-parallel has 'set -euo pipefail'"
else
  fail "pre-push-parallel does NOT have 'set -euo pipefail'"
fi
echo ""

# Verify: pre-push-parallel run_rustdoc captures exit codes
echo "--- Verify: run_rustdoc exit code capture in pre-push-parallel ---"
if grep -A20 'run_rustdoc()' .githooks/pre-push-parallel | grep -q 'return \$rc'; then
  pass "run_rustdoc returns exit code"
else
  fail "run_rustdoc does NOT return exit code"
fi
echo ""

# Verify: pre-push-parallel run_patterns captures output
echo "--- Verify: run_patterns output capture in pre-push-parallel ---"
if grep -A30 'run_patterns()' .githooks/pre-push-parallel | grep -q 'match_output'; then
  pass "run_patterns captures match output"
else
  fail "run_patterns does NOT capture match output"
fi
echo ""

# Verify: pre-push-sequential uses +$PINNED
echo "--- Verify: Toolchain pinning in pre-push-sequential ---"
if grep -E 'cargo \+"\$PINNED"' .githooks/pre-push-sequential | grep -q 'fmt'; then
  pass "pre-push-sequential fmt uses +\$PINNED"
else
  fail "pre-push-sequential fmt does NOT use +\$PINNED"
fi
if grep -E 'cargo \+"\$PINNED"' .githooks/pre-push-sequential | grep -q 'clippy'; then
  pass "pre-push-sequential clippy uses +\$PINNED"
else
  fail "pre-push-sequential clippy does NOT use +\$PINNED"
fi
echo ""

# Verify: pre-push-sequential has toolchain check
echo "--- Verify: Toolchain check in pre-push-sequential ---"
if grep -q 'rustup toolchain list' .githooks/pre-push-sequential && grep -q 'missing toolchain' .githooks/pre-push-sequential; then
  pass "pre-push-sequential checks if pinned toolchain is installed"
else
  fail "pre-push-sequential does NOT check if pinned toolchain is installed"
fi
echo ""

# Verify: pre-commit uses STAGED variable
echo "--- Verify: STAGED variable usage in pre-commit ---"
if grep -q 'echo "\$STAGED"' .githooks/pre-commit && grep -q 'PRNG_FILE' .githooks/pre-commit; then
  pass "pre-commit uses STAGED variable for PRNG check"
else
  fail "pre-commit does NOT use STAGED variable for PRNG check"
fi
echo ""

# Verify: sweep-stale-artifacts.sh SIZE fallback
echo "--- Verify: SIZE fallback in sweep-stale-artifacts.sh ---"
if grep -q 'echo "unknown"' scripts/sweep-stale-artifacts.sh || grep -q '|| echo' scripts/sweep-stale-artifacts.sh; then
  pass "sweep-stale-artifacts.sh has SIZE fallback"
else
  fail "sweep-stale-artifacts.sh does NOT have SIZE fallback"
fi
echo ""

# Summary
echo "=== Summary ==="
echo "PASS: $PASS"
echo "FAIL: $FAIL"
if [[ $FAIL -gt 0 ]]; then
  echo ""
  echo "⚠️  $FAIL issue(s) need to be fixed"
  exit 1
else
  echo ""
  echo "✅ All issues verified/fixed!"
  exit 0
fi
