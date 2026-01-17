#!/usr/bin/env bash
# SPDX-License-Identifier: Apache-2.0
# © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
# Test script to verify code review issues have been fixed
set -uo pipefail

cd "$(dirname "${BASH_SOURCE[0]}")/../.." || exit 1

PASS=0
FAIL=0

pass() { echo "  ✓ PASS: $1"; PASS=$((PASS+1)); }
fail() { echo "  ✗ FAIL: $1"; FAIL=$((FAIL+1)); }

echo "=== Testing Code Review Fixes ==="
echo

# Issue 1: pre-push-parallel consistent toolchain
echo "[Issue 1] pre-push-parallel: all stages should use pinned toolchain"
if grep -A2 'run_fmt()' .githooks/pre-push-parallel | grep 'cargo +"$PINNED"' >/dev/null 2>&1; then
  pass "run_fmt uses pinned toolchain"
else
  fail "run_fmt doesn't use +\"\$PINNED\""
fi
if grep -A2 'run_clippy()' .githooks/pre-push-parallel | grep 'cargo +"$PINNED"' >/dev/null 2>&1; then
  pass "run_clippy uses pinned toolchain"
else
  fail "run_clippy doesn't use +\"\$PINNED\""
fi
if grep -A2 'run_tests()' .githooks/pre-push-parallel | grep 'cargo +"$PINNED"' >/dev/null 2>&1; then
  pass "run_tests uses pinned toolchain"
else
  fail "run_tests doesn't use +\"\$PINNED\""
fi
echo

# Issue 2: trap kills background processes
echo "[Issue 2] pre-push-parallel: trap should kill background processes"
if grep -E "jobs -p.*xargs.*kill|pkill -P" .githooks/pre-push-parallel >/dev/null 2>&1; then
  pass "trap kills background jobs"
else
  fail "trap doesn't kill background jobs"
fi
if grep -E "trap.*EXIT.*INT.*TERM|trap.*INT.*TERM.*EXIT" .githooks/pre-push-parallel >/dev/null 2>&1; then
  pass "trap handles EXIT, INT, TERM signals"
else
  fail "trap doesn't handle all interrupt signals"
fi
echo

# Issue 3: toolchain check before using cargo +"$PINNED"
echo "[Issue 3] pre-push-parallel: should check toolchain exists"
if grep -E 'rustup toolchain list.*grep.*PINNED' .githooks/pre-push-parallel >/dev/null 2>&1; then
  pass "script checks if pinned toolchain is installed"
else
  fail "script doesn't verify pinned toolchain exists"
fi
echo

# Issue 4: run_rustdoc propagates exit codes
echo "[Issue 4] pre-push-parallel: run_rustdoc should propagate failures"
if grep -A20 'run_rustdoc()' .githooks/pre-push-parallel | grep -E 'return \$rc|rc=1' >/dev/null 2>&1; then
  pass "run_rustdoc captures and returns exit codes"
else
  fail "run_rustdoc always exits 0"
fi
echo

# Issue 5: mktemp validation / set -e
echo "[Issue 5] pre-push-parallel: should handle mktemp failures"
if grep -E '^set -e' .githooks/pre-push-parallel >/dev/null 2>&1; then
  pass "set -e ensures mktemp failure exits"
else
  fail "mktemp failure not handled"
fi
echo

# Issue 6: run_patterns shows rg output
echo "[Issue 6] pre-push-parallel: run_patterns should show matched lines"
# Check that rg output is captured into a variable and echoed
if grep -A20 'run_patterns()' .githooks/pre-push-parallel | grep -E 'match_output=.*rg' >/dev/null 2>&1; then
  if grep -A25 'run_patterns()' .githooks/pre-push-parallel | grep 'echo.*match_output' >/dev/null 2>&1; then
    pass "rg output is captured and displayed"
  else
    fail "rg output is captured but not displayed"
  fi
else
  fail "rg output is not captured"
fi
echo

# Issue 7: sweep-stale-artifacts SIZE safe
echo "[Issue 7] sweep-stale-artifacts.sh: SIZE assignment should be safe"
if grep -E 'SIZE=\$\(\s*\{.*\|\| echo' scripts/sweep-stale-artifacts.sh >/dev/null 2>&1; then
  pass "SIZE assignment has fallback"
else
  fail "SIZE assignment can abort script if du fails"
fi
echo

# Issue 8: pre-commit STAGED usage
echo "[Issue 8] pre-commit: should use cached STAGED variable"
if grep -E 'echo.*STAGED.*grep.*PRNG_FILE' .githooks/pre-commit >/dev/null 2>&1; then
  pass "PRNG check uses cached \$STAGED"
else
  fail "PRNG check doesn't use \$STAGED correctly"
fi
echo

# Issue 9: ban-nondeterminism allowlist
echo "[Issue 9] ban-nondeterminism: wsc files should be in allowlist"
if grep -E 'wsc/mod\.rs' .ban-nondeterminism-allowlist >/dev/null 2>&1 && \
   grep -E 'wsc/view\.rs' .ban-nondeterminism-allowlist >/dev/null 2>&1; then
  pass "wsc files are in allowlist"
else
  fail "wsc files are not in allowlist"
fi
echo

# Issue 10: ban-nondeterminism in pre-push
echo "[Issue 10] pre-push-parallel: should run ban-nondeterminism check"
if grep -E 'run_determinism|ban-nondeterminism' .githooks/pre-push-parallel >/dev/null 2>&1; then
  pass "ban-nondeterminism is integrated into pre-push"
else
  fail "ban-nondeterminism is not in pre-push"
fi
echo

echo "=== Summary ==="
echo "PASS: $PASS"
echo "FAIL: $FAIL"
echo

if [[ $FAIL -gt 0 ]]; then
  echo "Some issues still need fixing."
  exit 1
else
  echo "All issues have been addressed!"
  exit 0
fi
