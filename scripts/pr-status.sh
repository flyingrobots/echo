#!/usr/bin/env bash
# SPDX-License-Identifier: Apache-2.0
# © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
set -euo pipefail

export GH_PAGER=cat

SELECTOR="${1:-${PR:-}}"

gh_run() {
  local output
  local rc
  set +e
  output="$("$@" 2>&1)"
  rc=$?
  set -e
  if [[ "$rc" -ne 0 ]]; then
    if printf '%s\n' "$output" | grep -Eqi 'auth|authentication|not logged in'; then
      echo 'Auth error—run `gh auth login` and retry.' >&2
      return 1
    fi
    printf '%s\n' "$output" >&2
    return "$rc"
  fi
  printf '%s' "$output"
}

gh_run_checks() {
  local output
  local rc
  set +e
  output="$("$@" 2>&1)"
  rc=$?
  set -e
  if [[ "$rc" -ne 0 && "$rc" -ne 8 ]]; then
    if printf '%s\n' "$output" | grep -Eqi 'auth|authentication|not logged in'; then
      echo 'Auth error—run `gh auth login` and retry.' >&2
      return 1
    fi
    printf '%s\n' "$output" >&2
    return "$rc"
  fi
  printf '%s' "$output"
}

if [[ -n "$SELECTOR" ]]; then
  if ! VIEW_JSON="$(gh_run gh pr view "$SELECTOR" --json number,url,headRefOid,reviewDecision,mergeStateStatus)"; then
    exit 1
  fi
  if ! CHECKS_JSON="$(gh_run_checks gh pr checks "$SELECTOR" --json name,bucket,state)"; then
    exit 1
  fi
else
  if ! VIEW_JSON="$(gh_run gh pr view --json number,url,headRefOid,reviewDecision,mergeStateStatus)"; then
    exit 1
  fi
  if ! CHECKS_JSON="$(gh_run_checks gh pr checks --json name,bucket,state)"; then
    exit 1
  fi
fi

read -r PR_NUMBER PR_URL HEAD_SHA REVIEW_DECISION MERGE_STATE <<EOF
$(VIEW_JSON="$VIEW_JSON" python3 -c '
import json
import os

data = json.loads(os.environ["VIEW_JSON"])
print(
    data["number"],
    data["url"],
    data["headRefOid"][:12],
    data.get("reviewDecision") or "NONE",
    data.get("mergeStateStatus") or "UNKNOWN",
)
')
EOF

read -r PR_OWNER PR_NAME <<EOF
$(PR_URL="$PR_URL" python3 -c '
import os
from urllib.parse import urlparse

parts = [part for part in urlparse(os.environ["PR_URL"]).path.split("/") if part]
if len(parts) < 4 or parts[2] != "pull":
    raise SystemExit(f"unexpected PR URL: {os.environ['PR_URL']}")
print(parts[0], parts[1])
')
EOF

UNRESOLVED_THREADS=0
THREADS_CURSOR=""
while :; do
  THREAD_ARGS=(-F owner="$PR_OWNER" -F name="$PR_NAME" -F number="$PR_NUMBER")
  if [[ -n "$THREADS_CURSOR" ]]; then
    THREAD_ARGS+=(-F cursor="$THREADS_CURSOR")
  fi
  if ! THREADS_JSON="$(
    gh_run gh api graphql \
      "${THREAD_ARGS[@]}" \
      -f query='query($owner:String!, $name:String!, $number:Int!, $cursor:String) { repository(owner:$owner, name:$name) { pullRequest(number:$number) { reviewThreads(first:100, after:$cursor) { nodes { isResolved } pageInfo { hasNextPage endCursor } } } } }'
  )"; then
    exit 1
  fi
  if ! THREAD_PAGE="$(
    THREADS_JSON="$THREADS_JSON" python3 -c '
import json
import os

data = json.loads(os.environ["THREADS_JSON"])
review_threads = data["data"]["repository"]["pullRequest"]["reviewThreads"]
threads = review_threads["nodes"]
page_info = review_threads["pageInfo"]
print(
    sum(1 for thread in threads if not thread["isResolved"]),
    int(bool(page_info["hasNextPage"])),
    page_info["endCursor"] or "",
)
'
  )"; then
    exit 1
  fi
  read -r PAGE_UNRESOLVED PAGE_HAS_NEXT PAGE_CURSOR <<EOF
$THREAD_PAGE
EOF
  UNRESOLVED_THREADS=$((UNRESOLVED_THREADS + PAGE_UNRESOLVED))
  if [[ "$PAGE_HAS_NEXT" != "1" ]]; then
    break
  fi
  THREADS_CURSOR="$PAGE_CURSOR"
done

CHECK_GROUPS="$(
  CHECKS_JSON="$CHECKS_JSON" python3 -c '
import json
import os

data = json.loads(os.environ["CHECKS_JSON"])
groups = {"fail": [], "pending": [], "pass": [], "skipping": [], "cancel": []}
for item in data:
    groups.setdefault(item.get("bucket", "unknown"), []).append(item["name"])
for bucket in groups:
    groups[bucket].sort()
for bucket in ("fail", "pending", "pass", "skipping", "cancel"):
    names = groups.get(bucket, [])
    if names:
        print(bucket + "\t" + "\t".join(names))
'
)"

bucket_lines() {
  local bucket="$1"
  printf '%s\n' "$CHECK_GROUPS" | awk -F '\t' -v bucket="$bucket" '$1 == bucket {for (i = 2; i <= NF; i++) print $i}'
}

print_group() {
  local bucket="$1"
  local heading="$2"
  local lines
  lines="$(bucket_lines "$bucket")"
  if [[ -z "$lines" ]]; then
    return
  fi
  local count
  count="$(printf '%s\n' "$lines" | awk 'NF {count++} END {print count+0}')"
  echo
  echo "${heading} (${count})"
  printf '%s\n' "$lines" | sed 's/^/- /'
}

join_lines_csv() {
  local lines="$1"
  if [[ -z "$lines" ]]; then
    return
  fi
  printf '%s\n' "$lines" | python3 -c '
import sys

values = [line.strip() for line in sys.stdin if line.strip()]
print(", ".join(values))
'
}

print_blockers() {
  local blockers=()
  local failing_checks pending_checks
  failing_checks="$(bucket_lines fail)"
  pending_checks="$(bucket_lines pending)"

  if [[ "$UNRESOLVED_THREADS" -gt 0 ]]; then
    blockers+=("unresolved review threads: ${UNRESOLVED_THREADS}")
  fi
  if [[ -n "$failing_checks" ]]; then
    blockers+=("failing checks: $(join_lines_csv "$failing_checks")")
  fi
  if [[ -n "$pending_checks" ]]; then
    blockers+=("pending checks: $(join_lines_csv "$pending_checks")")
  fi
  if [[ "$REVIEW_DECISION" != "APPROVED" ]]; then
    blockers+=("review decision: ${REVIEW_DECISION}")
  fi
  if [[ "$MERGE_STATE" != "CLEAN" && "$MERGE_STATE" != "HAS_HOOKS" ]]; then
    blockers+=("merge state: ${MERGE_STATE}")
  fi

  echo "Current blockers:"
  if [[ "${#blockers[@]}" -eq 0 ]]; then
    echo "- none detected"
    return
  fi
  for blocker in "${blockers[@]}"; do
    echo "- ${blocker}"
  done
}

echo "PR #${PR_NUMBER}"
echo "URL: ${PR_URL}"
echo "Head SHA: ${HEAD_SHA}"
echo "Unresolved threads: ${UNRESOLVED_THREADS}"
echo "Review decision: ${REVIEW_DECISION}"
echo "Merge state: ${MERGE_STATE}"
print_blockers

print_group fail "Failing checks"
print_group pending "Pending checks"
print_group pass "Passing checks"
print_group skipping "Skipped checks"
print_group cancel "Cancelled checks"
