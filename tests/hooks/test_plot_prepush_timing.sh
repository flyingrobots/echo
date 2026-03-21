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

tmp="$(mktemp)"
cleanup() {
  rm -f "$tmp"
}
trap cleanup EXIT

cat >"$tmp" <<'EOF'
{"variant":"sequential","duration":9.5,"exit":0}
{"record_type":"run","mode":"full","elapsed_seconds":2.5,"exit_status":0}
{"record_type":"run","mode":"fast","elapsed_seconds":1.0,"exit_status":0}
EOF

echo "=== plot-prepush timing ==="
echo

output="$(node scripts/plot-prepush-timing.mjs "$tmp")"

if printf '%s\n' "$output" | grep -q 'full'; then
  pass "plotter includes current run-record modes"
else
  fail "plotter should include current run-record modes"
  printf '%s\n' "$output"
fi

if printf '%s\n' "$output" | grep -q 'fast'; then
  pass "plotter renders additional current modes from the same file"
else
  fail "plotter should render additional current modes"
  printf '%s\n' "$output"
fi

if printf '%s\n' "$output" | grep -q 'sequential'; then
  fail "plotter should prefer current run records over legacy rows in mixed files"
  printf '%s\n' "$output"
else
  pass "plotter ignores legacy rows when current run records are present"
fi

echo
echo "PASS: $PASS"
echo "FAIL: $FAIL"

if [[ "$FAIL" -ne 0 ]]; then
  exit 1
fi
