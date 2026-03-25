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

run_with_fake_gh() {
  local fixture="$1"
  local tmp
  local repo_root
  repo_root="$(pwd)"
  tmp="$(mktemp -d)"
  cleanup() {
    rm -rf "$tmp"
  }
  trap cleanup RETURN
  mkdir -p "$tmp/bin"

  case "$fixture" in
    success)
      cat >"$tmp/bin/gh" <<'EOF'
#!/usr/bin/env bash
set -euo pipefail
if [[ "${1:-}" == "pr" && "${2:-}" == "view" ]]; then
  cat <<'JSON'
{"number":302,"url":"https://github.com/flyingrobots/echo/pull/302","headRefOid":"123456789abcdeffedcba9876543210abcdef123","reviewDecision":"APPROVED","mergeStateStatus":"CLEAN"}
JSON
  exit 0
fi
if [[ "${1:-}" == "pr" && "${2:-}" == "checks" ]]; then
  cat <<'JSON'
[{"name":"Tests","bucket":"pass","state":"SUCCESS"},{"name":"Clippy","bucket":"pending","state":"PENDING"},{"name":"Determinism Guards","bucket":"fail","state":"FAILURE"}]
JSON
  exit 0
fi
if [[ "${1:-}" == "api" && "${2:-}" == "graphql" ]]; then
  if [[ "$*" != *"owner=flyingrobots"* || "$*" != *"name=echo"* ]]; then
    echo "expected repository owner/name arguments in graphql query" >&2
    exit 1
  fi
  if [[ "$*" == *"cursor=page-2"* ]]; then
    cat <<'JSON'
{"data":{"repository":{"pullRequest":{"reviewThreads":{"nodes":[{"isResolved":false}],"pageInfo":{"hasNextPage":false,"endCursor":null}}}}}}
JSON
    exit 0
  fi
  cat <<'JSON'
{"data":{"repository":{"pullRequest":{"reviewThreads":{"nodes":[{"isResolved":true},{"isResolved":false}],"pageInfo":{"hasNextPage":true,"endCursor":"page-2"}}}}}}
JSON
  exit 0
fi
echo "unexpected gh invocation: $*" >&2
exit 1
EOF
      ;;
    clean)
      cat >"$tmp/bin/gh" <<'EOF'
#!/usr/bin/env bash
set -euo pipefail
if [[ "${1:-}" == "pr" && "${2:-}" == "view" ]]; then
  cat <<'JSON'
{"number":308,"url":"https://github.com/flyingrobots/echo/pull/308","headRefOid":"abcdef1234567890abcdef1234567890abcdef12","reviewDecision":"APPROVED","mergeStateStatus":"CLEAN"}
JSON
  exit 0
fi
if [[ "${1:-}" == "pr" && "${2:-}" == "checks" ]]; then
  cat <<'JSON'
[{"name":"Tests","bucket":"pass","state":"SUCCESS"},{"name":"Clippy","bucket":"pass","state":"SUCCESS"}]
JSON
  exit 0
fi
if [[ "${1:-}" == "api" && "${2:-}" == "graphql" ]]; then
  cat <<'JSON'
{"data":{"repository":{"pullRequest":{"reviewThreads":{"nodes":[{"isResolved":true}],"pageInfo":{"hasNextPage":false,"endCursor":null}}}}}}
JSON
  exit 0
fi
echo "unexpected gh invocation: $*" >&2
exit 1
EOF
      ;;
    auth-error)
      cat >"$tmp/bin/gh" <<'EOF'
#!/usr/bin/env bash
set -euo pipefail
echo "gh: authentication required" >&2
exit 1
EOF
      ;;
    pagination-parse-error)
      cat >"$tmp/bin/gh" <<'EOF'
#!/usr/bin/env bash
set -euo pipefail
if [[ "${1:-}" == "pr" && "${2:-}" == "view" ]]; then
  cat <<'JSON'
{"number":302,"url":"https://github.com/flyingrobots/echo/pull/302","headRefOid":"123456789abcdeffedcba9876543210abcdef123","reviewDecision":"APPROVED","mergeStateStatus":"CLEAN"}
JSON
  exit 0
fi
if [[ "${1:-}" == "pr" && "${2:-}" == "checks" ]]; then
  cat <<'JSON'
[{"name":"Tests","bucket":"pass","state":"SUCCESS"}]
JSON
  exit 0
fi
if [[ "${1:-}" == "api" && "${2:-}" == "graphql" ]]; then
  if [[ "$*" == *"cursor=page-2"* ]]; then
    printf '%s\n' '{"data":{"repository":{"pullRequest":{"reviewThreads":'
    exit 0
  fi
  cat <<'JSON'
{"data":{"repository":{"pullRequest":{"reviewThreads":{"nodes":[{"isResolved":false}],"pageInfo":{"hasNextPage":true,"endCursor":"page-2"}}}}}}
JSON
  exit 0
fi
echo "unexpected gh invocation: $*" >&2
exit 1
EOF
      ;;
    *)
      echo "unknown fixture: $fixture" >&2
      exit 1
      ;;
  esac

  chmod +x "$tmp/bin/gh"
  (
    cd "$repo_root"
    PATH="$tmp/bin:$PATH" ./scripts/pr-status.sh 302 2>&1
  )
}

echo "=== Testing pr-status helper ==="
echo

status_output="$(run_with_fake_gh success)"
if printf '%s\n' "$status_output" | grep -q '^PR #302$'; then
  pass "pr-status reports the PR number"
else
  fail "pr-status should report the PR number"
  printf '%s\n' "$status_output"
fi
if printf '%s\n' "$status_output" | grep -q '^Head SHA: 123456789abc$'; then
  pass "pr-status truncates the head SHA consistently"
else
  fail "pr-status should print a 12-character head SHA"
  printf '%s\n' "$status_output"
fi
if printf '%s\n' "$status_output" | grep -q '^Unresolved threads: 2$'; then
  pass "pr-status reports unresolved review threads"
else
  fail "pr-status should count unresolved review threads"
  printf '%s\n' "$status_output"
fi
if printf '%s\n' "$status_output" | grep -q '^Review decision: APPROVED$'; then
  pass "pr-status reports review decision"
else
  fail "pr-status should report the review decision"
  printf '%s\n' "$status_output"
fi
if printf '%s\n' "$status_output" | grep -q '^Merge state: CLEAN$'; then
  pass "pr-status reports merge state"
else
  fail "pr-status should report merge state"
  printf '%s\n' "$status_output"
fi
if STATUS_OUTPUT="$status_output" python3 - <<'PY'
import os
import sys

lines = os.environ["STATUS_OUTPUT"].splitlines()
expected = [
    "Current blockers:",
    "- unresolved review threads: 2",
    "- failing checks: Determinism Guards",
    "- pending checks: Clippy",
]

indices = []
for item in expected:
    try:
        indices.append(lines.index(item))
    except ValueError:
        sys.exit(1)

sys.exit(0 if indices == sorted(indices) else 1)
PY
then
  pass "pr-status prints a concise blocker summary"
else
  fail "pr-status should summarize current blockers"
  printf '%s\n' "$status_output"
fi
if STATUS_OUTPUT="$status_output" python3 - <<'PY'
import os
import sys

lines = os.environ["STATUS_OUTPUT"].splitlines()

def heading_contains(heading, item):
    try:
        idx = lines.index(heading)
    except ValueError:
        return False
    for line in lines[idx + 1:]:
        if not line.strip():
            break
        if line == f"- {item}":
            return True
    return False

ok = (
    heading_contains("Failing checks (1)", "Determinism Guards")
    and heading_contains("Pending checks (1)", "Clippy")
    and heading_contains("Passing checks (1)", "Tests")
)
sys.exit(0 if ok else 1)
PY
then
  pass "pr-status groups checks by bucket"
else
  fail "pr-status should group checks by bucket"
  printf '%s\n' "$status_output"
fi

clean_output="$(run_with_fake_gh clean)"
if printf '%s\n' "$clean_output" | grep -q '^Current blockers:$' && printf '%s\n' "$clean_output" | grep -q '^- none detected$'; then
  pass "pr-status reports when no blockers are detected"
else
  fail "pr-status should report a clean readiness summary"
  printf '%s\n' "$clean_output"
fi

auth_output="$(run_with_fake_gh auth-error || true)"
if printf '%s\n' "$auth_output" | grep -q 'Auth error—run `gh auth login` and retry\.'; then
  pass "pr-status reports auth failures clearly"
else
  fail "pr-status should emit the auth guidance message"
  printf '%s\n' "$auth_output"
fi

if run_with_fake_gh pagination-parse-error >/dev/null 2>&1; then
  fail "pr-status should fail fast when paginated thread JSON cannot be parsed"
else
  pass "pr-status propagates pagination parse failures"
fi

echo
echo "PASS: $PASS"
echo "FAIL: $FAIL"

if [[ "$FAIL" -ne 0 ]]; then
  exit 1
fi
