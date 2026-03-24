#!/usr/bin/env bash
# SPDX-License-Identifier: Apache-2.0
# © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
set -euo pipefail

cd "$(dirname "${BASH_SOURCE[0]}")/../.." || exit 1

PASS=0
FAIL=0

tmpdir_directives="$(mktemp -d)"

pass() {
  echo "  PASS: $1"
  PASS=$((PASS + 1))
}

fail() {
  echo "  FAIL: $1"
  FAIL=$((FAIL + 1))
}

tmpdir="$(mktemp -d)"
output_file="$(mktemp)"

cleanup() {
  rm -rf "$tmpdir_directives"
  rm -rf "$tmpdir"
  rm -f "$output_file"
}
trap cleanup EXIT

echo "=== runtime schema validation ==="
echo

if node scripts/validate-runtime-schema-fragments.mjs >"$output_file" 2>&1; then
  pass "validator accepts the checked-in runtime schema fragments"
else
  fail "validator should accept the checked-in runtime schema fragments"
  cat "$output_file"
fi

cp schemas/runtime/*.graphql "$tmpdir_directives"/
cat <<'EOF' >"$tmpdir_directives/directive-safe.graphql"
# SPDX-License-Identifier: Apache-2.0 OR LicenseRef-MIND-UCAL-1.0
# © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>

type DirectiveSafeProbe {
    legacyField: String @deprecated(reason: "old")
}
EOF

if node scripts/validate-runtime-schema-fragments.mjs --dir "$tmpdir_directives" >"$output_file" 2>&1; then
  pass "validator accepts directive-bearing GraphQL fields"
else
  fail "validator should accept directive-bearing GraphQL fields"
  cat "$output_file"
fi

cp schemas/runtime/*.graphql "$tmpdir"/
sed 's/^[[:space:]]*scalar RunId[[:space:]]*$/scalar RemovedRunId/' \
  "$tmpdir/artifact-d-scheduler-results.graphql" \
  >"$tmpdir/artifact-d-scheduler-results.graphql.tmp"
mv \
  "$tmpdir/artifact-d-scheduler-results.graphql.tmp" \
  "$tmpdir/artifact-d-scheduler-results.graphql"

if node scripts/validate-runtime-schema-fragments.mjs --dir "$tmpdir" >"$output_file" 2>&1; then
  fail "validator should reject fragments with missing referenced types"
  cat "$output_file"
else
  if grep -q "missing referenced type RunId" "$output_file"; then
    pass "validator reports missing referenced types across fragment files"
  else
    fail "validator should explain which referenced type is missing"
    cat "$output_file"
  fi
fi

echo
echo "PASS: $PASS"
echo "FAIL: $FAIL"

if [[ "$FAIL" -ne 0 ]]; then
  exit 1
fi
