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

tmpdir="$(mktemp -d)"
output_file="$(mktemp)"

cleanup() {
  rm -rf "$tmpdir"
  rm -f "$output_file"
}
trap cleanup EXIT

mkdir -p \
  "$tmpdir/scripts" \
  "$tmpdir/docs/archive/tasks" \
  "$tmpdir/docs/assets/dags" \
  "$tmpdir/.cache/echo/deps"

cat >"$tmpdir/package.json" <<'EOF'
{
  "type": "module"
}
EOF

cp scripts/generate-dependency-dags.js "$tmpdir/scripts/generate-dependency-dags.js"
cp scripts/generate-tasks-dag.js "$tmpdir/scripts/generate-tasks-dag.js"
cp scripts/parse-tasks-dag.js "$tmpdir/scripts/parse-tasks-dag.js"
cp scripts/dag-utils.js "$tmpdir/scripts/dag-utils.js"

cat >"$tmpdir/.cache/echo/deps/open-issues.json" <<'EOF'
{
  "generated_at": "2026-03-23T00:00:00Z",
  "issues": [
    {
      "number": 1,
      "title": "Seed issue",
      "body": "",
      "labels": [],
      "milestone": null,
      "url": "https://example.com/issues/1"
    },
    {
      "number": 2,
      "title": "Dependent issue",
      "body": "",
      "labels": [],
      "milestone": null,
      "url": "https://example.com/issues/2"
    }
  ]
}
EOF

cat >"$tmpdir/.cache/echo/deps/milestones-all.json" <<'EOF'
{
  "generated_at": "2026-03-23T00:00:00Z",
  "milestones": []
}
EOF

cat >"$tmpdir/docs/assets/dags/deps-config.json" <<'EOF'
{
  "issue_edges": [],
  "milestone_edges": []
}
EOF

cat >"$tmpdir/docs/archive/tasks/TASKS-DAG.md" <<'EOF'
## [#2: Dependent issue](https://example.com/issues/2)

- Blocked by:
  - [#1: Seed issue](https://example.com/issues/1)
EOF

echo "=== dependency DAG default tasks source ==="
echo

if (
  cd "$tmpdir" &&
    node scripts/generate-dependency-dags.js >"$output_file" 2>&1
); then
  if grep -Eq 'i1 -> i2 \[[^]]*color="red"' "$tmpdir/docs/assets/dags/issue-deps.dot"; then
    pass "generator reads archived TASKS-DAG source by default"
  else
    fail "generator should render a reality-only edge from the archived TASKS-DAG source"
    if [[ -f "$tmpdir/docs/assets/dags/issue-deps.dot" ]]; then
      cat "$tmpdir/docs/assets/dags/issue-deps.dot"
    else
      cat "$output_file"
    fi
  fi
else
  fail "generator should succeed with only the archived TASKS-DAG source present"
  cat "$output_file"
fi

cat >"$tmpdir/docs/assets/dags/clusters-config.json" <<'EOF'
["   "]
EOF
cat >"$tmpdir/docs/archive/tasks/TASKS-DAG.md" <<'EOF'
## [#1: Alpha seed](https://example.com/issues/1)

- Blocks:
  - [#2: Beta dependent](https://example.com/issues/2)
EOF

echo
echo "=== tasks DAG cluster-prefix validation ==="
echo

if (
  cd "$tmpdir" &&
    node scripts/generate-tasks-dag.js >"$output_file" 2>&1
); then
  if grep -q 'clusters-config.json is invalid (expected array of non-empty strings); using defaults.' "$output_file" &&
    grep -q 'subgraph cluster_Misc' "$tmpdir/docs/assets/dags/tasks-dag.dot"; then
    pass "tasks DAG generator rejects blank cluster prefixes and falls back to defaults"
  else
    fail "tasks DAG generator should reject blank cluster prefixes and cluster unmatched nodes as Misc"
    cat "$output_file"
    cat "$tmpdir/docs/assets/dags/tasks-dag.dot"
  fi
else
  fail "tasks DAG generator should succeed when cluster config falls back to defaults"
  cat "$output_file"
fi

echo
echo "PASS: $PASS"
echo "FAIL: $FAIL"

if [[ "$FAIL" -ne 0 ]]; then
  exit 1
fi
