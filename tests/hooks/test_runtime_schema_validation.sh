#!/usr/bin/env bash
# SPDX-License-Identifier: Apache-2.0
# © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
set -euo pipefail

cd "$(dirname "${BASH_SOURCE[0]}")/../.." || exit 1

PASS=0
FAIL=0

tmpdir_directives="$(mktemp -d)"
tmpdir_prettier="$(mktemp -d)"

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
  rm -rf "$tmpdir_prettier"
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

mkdir -p \
  "$tmpdir_prettier/scripts" \
  "$tmpdir_prettier/schemas/runtime" \
  "$tmpdir_prettier/node_modules/.bin" \
  "$tmpdir_prettier/bin"
ln -s "$(pwd)/node_modules/graphql" "$tmpdir_prettier/node_modules/graphql"
cp scripts/validate-runtime-schema-fragments.mjs \
  "$tmpdir_prettier/scripts/validate-runtime-schema-fragments.mjs"
cat <<'EOF' >"$tmpdir_prettier/schemas/runtime/test.graphql"
# SPDX-License-Identifier: Apache-2.0 OR LicenseRef-MIND-UCAL-1.0
# © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>

scalar RuntimeProbe
EOF
cat <<'EOF' >"$tmpdir_prettier/node_modules/.bin/prettier"
#!/usr/bin/env bash
set -euo pipefail
if [[ "${1:-}" == "--version" ]]; then
  echo "3.8.1"
  exit 0
fi
cat
EOF
cat <<'EOF' >"$tmpdir_prettier/bin/npx"
#!/usr/bin/env bash
set -euo pipefail
echo "npx should not be called" >&2
exit 99
EOF
cat <<'EOF' >"$tmpdir_prettier/bin/pnpm"
#!/usr/bin/env bash
set -euo pipefail
echo "pnpm should not be called" >&2
exit 98
EOF
chmod +x \
  "$tmpdir_prettier/node_modules/.bin/prettier" \
  "$tmpdir_prettier/bin/npx" \
  "$tmpdir_prettier/bin/pnpm"

if (
  cd "$tmpdir_prettier" &&
    PATH="$tmpdir_prettier/bin:$PATH" \
    node scripts/validate-runtime-schema-fragments.mjs >"$output_file" 2>&1
); then
  pass "validator prefers pinned local prettier before npx or pnpm"
else
  fail "validator should prefer the pinned local prettier binary"
  cat "$output_file"
fi

echo
echo "PASS: $PASS"
echo "FAIL: $FAIL"

if [[ "$FAIL" -ne 0 ]]; then
  exit 1
fi
