#!/usr/bin/env bash
# SPDX-License-Identifier: Apache-2.0
# © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>

set -euo pipefail

usage() {
  cat <<'EOF'
Usage:
  .github/scripts/extract-actionable-comments.sh <PR_NUMBER> [--repo OWNER/REPO] [--out <path>] [--full]
  .github/scripts/extract-actionable-comments.sh <PR_NUMBER> [--repo OWNER/REPO] [--out <path>] [--full] [--all-sources]
  .github/scripts/extract-actionable-comments.sh <PR_NUMBER> [--repo OWNER/REPO] [--out <path>] [--full] [--include-conversation] [--include-reviews]

Purpose:
  Extract actionable PR feedback (CodeRabbitAI + humans) from:
  - PR review threads (inline review comments; diff-positioned, can become outdated)
  - optionally PR conversation comments (issue comments)
  - optionally PR review summaries (review bodies, e.g. "changes requested")

  Review threads are grouped by staleness; all sources support a lightweight ack
  convention via: ✅ Addressed in commit <sha>

Outputs:
  - Writes a Markdown report to stdout by default.
  - Also writes raw JSON + intermediate artifacts to /tmp.

Options:
  --repo OWNER/REPO        Override repo (default: current repo via `gh repo view`)
  --out <path>             Write the Markdown report to <path> (also prints to stdout)
  --full                   Include full comment bodies for actionable comments
  --include-conversation   Also include PR conversation (issue) comments
  --include-reviews        Also include PR review summaries (approve/request-changes bodies)
  --all-sources            Equivalent to: --include-conversation --include-reviews

Examples:
  .github/scripts/extract-actionable-comments.sh 176
  .github/scripts/extract-actionable-comments.sh 176 --out /tmp/pr-176-report.md
  .github/scripts/extract-actionable-comments.sh 176 --full
  .github/scripts/extract-actionable-comments.sh 176 --all-sources
EOF
}

require_cmd() {
  local cmd="$1"
  command -v "$cmd" >/dev/null 2>&1 || { echo "Missing dependency: $cmd" >&2; exit 2; }
}

fetch_paginated_json() {
  local api_path="$1"
  local out_json="$2"
  local out_err="$3"

  local attempt=1
  local delay_s=1
  while true; do
    if gh api "$api_path" --paginate > "$out_json" 2> "$out_err"; then
      break
    fi

    if [[ "$attempt" -ge 4 ]]; then
      echo "Error: Failed to fetch GitHub API '${api_path}' after ${attempt} attempts." >&2
      echo "Troubleshooting:" >&2
      echo "- Run: gh auth status" >&2
      echo "- Check rate limits / token scopes (GH_TOKEN) and retry later" >&2
      echo "- Verify repo/PR access permissions" >&2
      echo "- Check network connectivity" >&2
      echo >&2
      echo "gh api stderr (last attempt):" >&2
      sed -n '1,200p' "$out_err" >&2
      exit 1
    fi

    sleep "$delay_s"
    delay_s="$((delay_s * 2))"
    attempt="$((attempt + 1))"
  done

  if ! jq -e . "$out_json" >/dev/null 2>&1; then
    echo "Error: GitHub API returned invalid JSON in: ${out_json}" >&2
    echo "gh api stderr:" >&2
    sed -n '1,200p' "$out_err" >&2
    exit 1
  fi
}

PR_NUMBER="${1:-}"
shift || true # Prevent set -e exit when $1 is absent (no remaining args to shift).
if [[ -z "${PR_NUMBER}" ]]; then
  usage >&2
  exit 2
fi
if ! [[ "${PR_NUMBER}" =~ ^[0-9]+$ ]]; then
  echo "Error: PR_NUMBER must be numeric, got: ${PR_NUMBER}" >&2
  usage >&2
  exit 2
fi

REPO=""
OUT=""
FULL=0
INCLUDE_CONVERSATION=0
INCLUDE_REVIEWS=0

while [[ $# -gt 0 ]]; do
  case "$1" in
    --repo)
      if [[ -z "${2:-}" || "${2:-}" == -* ]]; then
        echo "Error: --repo requires a value in the form OWNER/REPO" >&2
        usage >&2
        exit 2
      fi
      REPO="$2"
      shift 2
      ;;
    --out)
      if [[ -z "${2:-}" || "${2:-}" == -* ]]; then
        echo "Error: --out requires a filesystem path" >&2
        usage >&2
        exit 2
      fi
      OUT="$2"
      shift 2
      ;;
    --full)
      FULL=1
      shift
      ;;
    --include-conversation)
      INCLUDE_CONVERSATION=1
      shift
      ;;
    --include-reviews)
      INCLUDE_REVIEWS=1
      shift
      ;;
    --all-sources)
      INCLUDE_CONVERSATION=1
      INCLUDE_REVIEWS=1
      shift
      ;;
    -h|--help)
      usage
      exit 0
      ;;
    *)
      echo "Unknown argument: $1" >&2
      usage >&2
      exit 2
      ;;
  esac
done

require_cmd gh
require_cmd jq

if [[ -z "$REPO" ]]; then
  REPO="$(gh repo view --json nameWithOwner --jq '.nameWithOwner')"
fi

OWNER="${REPO%/*}"
NAME="${REPO#*/}"
if [[ "$REPO" != */* || "$REPO" == */*/* || -z "$OWNER" || -z "$NAME" || "$OWNER" == "$NAME" ]]; then
  echo "Error: Invalid repo format '${REPO}'. Expected OWNER/REPO." >&2
  exit 2
fi

HEAD_SHA="$(gh pr view "$PR_NUMBER" --repo "$REPO" --json headRefOid --jq '.headRefOid')"
if [[ -z "$HEAD_SHA" || ! "$HEAD_SHA" =~ ^[0-9a-f]{7,}$ ]]; then
  echo "Error: Failed to determine PR head commit SHA for ${REPO}#${PR_NUMBER}" >&2
  exit 1
fi
HEAD7="${HEAD_SHA:0:7}"

TS="$(date +%s)-$$"
RAW_COMMITS="/tmp/pr-${PR_NUMBER}-commits-${TS}.json"
RAW_COMMITS_ERR="/tmp/pr-${PR_NUMBER}-commits-${TS}.err"
RAW_REVIEW="/tmp/pr-${PR_NUMBER}-review-comments-${TS}.json"
RAW_REVIEW_ERR="/tmp/pr-${PR_NUMBER}-review-comments-${TS}.err"
LATEST_REVIEW="/tmp/pr-${PR_NUMBER}-review-latest-${TS}.json"
RAW_CONVERSATION="/tmp/pr-${PR_NUMBER}-conversation-comments-${TS}.json"
RAW_CONVERSATION_ERR="/tmp/pr-${PR_NUMBER}-conversation-comments-${TS}.err"
LATEST_CONVERSATION="/tmp/pr-${PR_NUMBER}-conversation-latest-${TS}.json"
RAW_REVIEWS="/tmp/pr-${PR_NUMBER}-reviews-${TS}.json"
RAW_REVIEWS_ERR="/tmp/pr-${PR_NUMBER}-reviews-${TS}.err"
LATEST_REVIEWS="/tmp/pr-${PR_NUMBER}-reviews-latest-${TS}.json"
LATEST_ALL="/tmp/pr-${PR_NUMBER}-latest-${TS}.json"
REPORT="/tmp/pr-${PR_NUMBER}-report-${TS}.md"

fetch_paginated_json "repos/${OWNER}/${NAME}/pulls/${PR_NUMBER}/commits" "$RAW_COMMITS" "$RAW_COMMITS_ERR"
VALID_COMMITS_JSON="$(jq -c '[ .[] | (.sha // "")[0:7] | select(length == 7) ] | unique' "$RAW_COMMITS")"
if [[ -z "$VALID_COMMITS_JSON" || "$VALID_COMMITS_JSON" == "[]" ]]; then
  VALID_COMMITS_JSON="$(jq -nc --arg head "$HEAD7" '[ $head ]')"
fi

fetch_paginated_json "repos/${OWNER}/${NAME}/pulls/${PR_NUMBER}/comments" "$RAW_REVIEW" "$RAW_REVIEW_ERR"
if [[ "$INCLUDE_CONVERSATION" -eq 1 ]]; then
  fetch_paginated_json "repos/${OWNER}/${NAME}/issues/${PR_NUMBER}/comments" "$RAW_CONVERSATION" "$RAW_CONVERSATION_ERR"
fi
if [[ "$INCLUDE_REVIEWS" -eq 1 ]]; then
  fetch_paginated_json "repos/${OWNER}/${NAME}/pulls/${PR_NUMBER}/reviews" "$RAW_REVIEWS" "$RAW_REVIEWS_ERR"
fi

# Normalize review-thread comments (top-level only) and detect ack markers in replies.
FILTER_COMMON="/tmp/pr-${PR_NUMBER}-jq-common-${TS}.jq"
FILTER_REVIEW="/tmp/pr-${PR_NUMBER}-jq-review-thread-${TS}.jq"
FILTER_CONVERSATION="/tmp/pr-${PR_NUMBER}-jq-conversation-${TS}.jq"
FILTER_REVIEWS="/tmp/pr-${PR_NUMBER}-jq-review-summaries-${TS}.jq"
FILTER_REVIEW_FULL="/tmp/pr-${PR_NUMBER}-jq-review-thread-full-${TS}.jq"
FILTER_CONVERSATION_FULL="/tmp/pr-${PR_NUMBER}-jq-conversation-full-${TS}.jq"
FILTER_REVIEWS_FULL="/tmp/pr-${PR_NUMBER}-jq-review-summaries-full-${TS}.jq"

cat > "$FILTER_COMMON" <<'JQ'
def is_bot_user(u):
  (u | type) == "object"
  and (
    (u.type // "") == "Bot"
    or ((u.login // "") | endswith("[bot]"))
  );

# An ack marker is considered valid only when:
# - authored by a non-bot user (prevents false positives from CodeRabbit templates), and
# - includes a commit SHA that is actually part of the PR (reduces accidental matches).
def ack_commit(body):
  if (body | type) != "string" then null
  else
    (try
      (body
        | capture("(?m)^[\\s>]*✅ Addressed in commit (?<commit>[0-9a-f]{7,40})\\b")
        | .commit
        | ascii_downcase
        | .[0:7]
      )
    catch null)
  end;

def has_ack_marker(body; user):
  (is_bot_user(user) | not)
  and (ack_commit(body) as $c
    | $c != null
    and ($valid_commits | index($c)) != null
  );

def normalize_title(body):
  ((body
    | split("\n")
    | map(select(. != ""))
    | .[0] // "UNTITLED"
  )
  | gsub("\\*\\*"; "")
  | .[0:80]);

def priority_from_body(body):
  if (body | test("\\bP0\\b|badge/P0-|🔴|Critical"; "i")) then "P0"
  elif (body | test("\\bP1\\b|badge/P1-|🟠|Major"; "i")) then "P1"
  elif (body | test("\\bP2\\b|badge/P2-|🟡|Minor"; "i")) then "P2"
  else "P3"
  end;

def likely_actionable(body):
  (body | type) == "string"
  and (body | test("\\bP[0-3]\\b|\\bTODO\\b|\\bFIXME\\b|\\bnit\\b|suggest|\\bshould\\b|\\bconsider\\b|blocker|\\bbug\\b|error|fail|typo|rename|missing|clarify|doc(s|ument)?|\\btests?\\b|panic|crash|security|\\bdetermin"; "i"));
JQ

cat > "$FILTER_REVIEW" <<'JQ'
# Replies are returned in the same list, with `in_reply_to_id` set.
def ack_by_reply:
  reduce .[] as $c ({}; if ($c.in_reply_to_id != null and has_ack_marker($c.body; $c.user)) then .[($c.in_reply_to_id | tostring)] = true else . end);

(ack_by_reply) as $replies |
[ .[]
  | select(.in_reply_to_id == null)
  | {
      id,
      author: (.user.login // "unknown"),
      author_is_bot: (
        (.user.type // "") == "Bot"
        or ((.user.login // "") | endswith("[bot]"))
      ),
      url: .html_url,
      source: "review_thread",
      path,
      line,
      position,
      original_position,
      head_commit: $head,
      comment_commit: (.commit_id[0:7]),
      original_commit: (.original_commit_id[0:7]),
      is_on_head: (.commit_id[0:7] == $head),
      is_visible_on_head_diff: (.position != null),
      is_outdated: (.position == null),
      is_moved: (.commit_id != .original_commit_id),
      has_ack: ($replies[(.id | tostring)] // false),
      is_actionable: true,
      priority: priority_from_body(.body),
      title: normalize_title(.body),
      body: .body
    }
]
JQ

cat "$FILTER_COMMON" "$FILTER_REVIEW" > "$FILTER_REVIEW_FULL"
jq --arg head "$HEAD7" --argjson valid_commits "$VALID_COMMITS_JSON" -f "$FILTER_REVIEW_FULL" "$RAW_REVIEW" > "$LATEST_REVIEW"

if [[ "$INCLUDE_CONVERSATION" -eq 1 ]]; then
  cat > "$FILTER_CONVERSATION" <<'JQ'
def is_html_comment(body):
  (body | type) == "string"
  and (body | test("^\\s*<!--"));

[ .[]
  | select((.body // "") | test("\\S"))
  | select(is_html_comment(.body) | not)
  | (.user // {}) as $u
  | (
      ($u.type // "") == "Bot"
      or (($u.login // "") | endswith("[bot]"))
    ) as $is_bot
  | {
      id,
      author: ($u.login // "unknown"),
      author_is_bot: $is_bot,
      url: .html_url,
      source: "conversation",
      path: null,
      line: null,
      position: null,
      original_position: null,
      head_commit: null,
      comment_commit: null,
      original_commit: null,
      is_on_head: false,
      is_visible_on_head_diff: false,
      is_outdated: false,
      is_moved: false,
      has_ack: has_ack_marker(.body; $u),
      is_actionable: (($is_bot | not) and likely_actionable(.body)),
      priority: priority_from_body(.body),
      title: normalize_title(.body),
      body: .body
    }
]
JQ
  cat "$FILTER_COMMON" "$FILTER_CONVERSATION" > "$FILTER_CONVERSATION_FULL"
  jq --argjson valid_commits "$VALID_COMMITS_JSON" -f "$FILTER_CONVERSATION_FULL" "$RAW_CONVERSATION" > "$LATEST_CONVERSATION"
else
  printf '%s\n' '[]' > "$LATEST_CONVERSATION"
fi

if [[ "$INCLUDE_REVIEWS" -eq 1 ]]; then
  cat > "$FILTER_REVIEWS" <<'JQ'
[ .[]
  | select((.body // "") | test("\\S"))
  | (.user // {}) as $u
  | (
      ($u.type // "") == "Bot"
      or (($u.login // "") | endswith("[bot]"))
    ) as $is_bot
  | {
      id,
      author: ($u.login // "unknown"),
      author_is_bot: $is_bot,
      url: .html_url,
      source: "review_summary",
      review_state: (.state // "UNKNOWN"),
      path: null,
      line: null,
      position: null,
      original_position: null,
      head_commit: null,
      comment_commit: null,
      original_commit: null,
      is_on_head: false,
      is_visible_on_head_diff: false,
      is_outdated: false,
      is_moved: false,
      has_ack: has_ack_marker(.body; $u),
      is_actionable: (
        (($is_bot | not) and ((.state // "") == "CHANGES_REQUESTED"))
        or (($is_bot | not) and likely_actionable(.body))
      ),
      priority: priority_from_body(.body),
      title: normalize_title(.body),
      body: .body
    }
]
JQ
  cat "$FILTER_COMMON" "$FILTER_REVIEWS" > "$FILTER_REVIEWS_FULL"
  jq --argjson valid_commits "$VALID_COMMITS_JSON" -f "$FILTER_REVIEWS_FULL" "$RAW_REVIEWS" > "$LATEST_REVIEWS"
else
  printf '%s\n' '[]' > "$LATEST_REVIEWS"
fi

jq -s 'add' "$LATEST_REVIEW" "$LATEST_CONVERSATION" "$LATEST_REVIEWS" > "$LATEST_ALL"

if ! IFS=$'\t' read -r \
  total_count \
  total_actionable_count \
  needs_attention_count \
  needs_attention_human_count \
  needs_attention_bot_count \
  on_head_attention_count \
  outdated_attention_count \
  conversation_attention_count \
  review_summary_attention_count \
  unclassified_count \
  moved_count \
  ack_count \
  < <(
    jq -r '
      [
        length,
        ([.[] | select(.is_actionable == true)] | length),
        ([.[] | select(.is_actionable == true and .has_ack == false)] | length),
        ([.[] | select(.is_actionable == true and .has_ack == false and .author_is_bot == false)] | length),
        ([.[] | select(.is_actionable == true and .has_ack == false and .author_is_bot == true)] | length),
        ([.[] | select(.source == "review_thread" and .is_visible_on_head_diff == true and .is_actionable == true and .has_ack == false)] | length),
        ([.[] | select(.source == "review_thread" and .is_outdated == true and .is_actionable == true and .has_ack == false)] | length),
        ([.[] | select(.source == "conversation" and .is_actionable == true and .has_ack == false)] | length),
        ([.[] | select(.source == "review_summary" and .is_actionable == true and .has_ack == false)] | length),
        ([.[] | select(.is_actionable == false and .has_ack == false)] | length),
        ([.[] | select(.source == "review_thread" and .is_moved == true)] | length),
        ([.[] | select(.has_ack == true)] | length)
      ] | @tsv
    ' "$LATEST_ALL"
  ); then
  echo "Error: Failed to compute report counts from: ${LATEST_ALL}" >&2
  exit 1
fi

{
  echo "# PR Review Actionables — PR #${PR_NUMBER}"
  echo
  echo "- Repo: \`${REPO}\`"
  echo "- PR head: \`${HEAD7}\`"
  echo "- Generated: \`$(date -u +"%Y-%m-%dT%H:%M:%SZ")\`"
  echo

  echo "## Summary"
  echo
  echo "- Total extracted items: **${total_count}**"
  echo "- Total actionable items: **${total_actionable_count}**"
  echo "- Needs attention (actionable + unacknowledged): **${needs_attention_count}**"
  echo "  - Human reviewers: **${needs_attention_human_count}**"
  echo "  - Bots (including CodeRabbitAI): **${needs_attention_bot_count}**"
  echo "  - Review threads (on head diff): **${on_head_attention_count}**"
  echo "  - Review threads (outdated): **${outdated_attention_count}**"
  echo "  - PR conversation: **${conversation_attention_count}**"
  echo "  - Review summaries: **${review_summary_attention_count}**"
  echo "- Unclassified (unacknowledged): **${unclassified_count}**"
  echo "- Acknowledged (✅ Addressed): **${ack_count}**"
  echo "- Moved by GitHub (review threads only; commit_id != original_commit_id): **${moved_count}**"
  echo

  echo "## Needs Attention (Review Threads — On Head Diff)"
  echo
  jq -r '
    .[]
    | select(.source == "review_thread" and .is_visible_on_head_diff == true and .is_actionable == true and .has_ack == false)
    | "- [ ] [\(.priority)] \(.path):\(.line // 1) — \(.title) (by @\(.author)) [id=\(.id)]"
  ' "$LATEST_ALL"
  echo

  echo "## Needs Attention (Review Threads — Outdated / Earlier Commits)"
  echo
  jq -r '
    .[]
    | select(.source == "review_thread" and .is_outdated == true and .is_actionable == true and .has_ack == false)
    | "- [ ] [\(.priority)] \(.path):\(.line // 1) — \(.title) (by @\(.author), comment commit: \(.comment_commit)) [id=\(.id)]"
  ' "$LATEST_ALL"
  echo

  echo "## Needs Attention (PR Conversation)"
  echo
  jq -r '
    .[]
    | select(.source == "conversation" and .is_actionable == true and .has_ack == false)
    | "- [ ] [\(.priority)] PR conversation — \(.title) (by @\(.author)) [id=\(.id)]"
  ' "$LATEST_ALL"
  echo

  echo "## Needs Attention (Review Summaries)"
  echo
  jq -r '
    .[]
    | select(.source == "review_summary" and .is_actionable == true and .has_ack == false)
    | "- [ ] [\(.priority)] Review \((.review_state // "UNKNOWN")) — \(.title) (by @\(.author)) [id=\(.id)]"
  ' "$LATEST_ALL"
  echo

  echo "## Unclassified (Conversation + Review Summaries)"
  echo
  jq -r '
    .[]
    | select((.source == "conversation" or .source == "review_summary") and .is_actionable == false and .has_ack == false)
    | "- [ ] [\(.priority)] \(.source) — \(.title) (by @\(.author)) [id=\(.id)]"
  ' "$LATEST_ALL"
  echo

  echo "## Acknowledged"
  echo
  jq -r '
    .[]
    | select(.has_ack == true)
    | "- [ ] \(.source) — \(.title) (by @\(.author), acknowledged) [id=\(.id)]"
  ' "$LATEST_ALL"
  echo

  echo "## Notes"
  echo
  echo "- \"Outdated\" means the review thread comment is no longer visible on the current head diff; it may still be actionable."
  echo "- Use \`✅ Addressed in commit <sha>\` replies (or edits) to close the loop and keep future extraction cheap."
  echo "- Conversation comments + review summaries are only included when requested; they are not diff-positioned like review threads."
  echo

  if [[ "$FULL" -eq 1 ]]; then
    echo "## Full Comment Bodies (Needs Attention)"
    echo
    jq -r '
      def pnum(p):
        if p == "P0" then 0
        elif p == "P1" then 1
        elif p == "P2" then 2
        else 3
        end;

      [ .[]
        | select(.is_actionable == true and .has_ack == false)
      ]
      | sort_by([.source, (.is_outdated | if . then 1 else 0 end), pnum(.priority), .author, (.path // ""), (.line // 0)])
      | .[]
      | "### [\(.priority)] \(.source) — \((.path // "PR")):\(.line // 1) [id=\(.id)]\n\n- Author: @\(.author)\n- URL: \(.url // "unknown")\n- Review state: \((.review_state // "n/a"))\n- Visible on head diff: \(.is_visible_on_head_diff)\n- Outdated: \(.is_outdated)\n- Comment commit: \((.comment_commit // "n/a"))\n\n````\n\(.body)\n````\n"
    ' "$LATEST_ALL"
  else
    echo "## Next Step"
    echo
    echo "Run with \`--full\` to include full comment bodies for the needs-attention set."
  fi

  echo
  echo "## Artifacts"
  echo
  echo "- Review-thread raw: \`${RAW_REVIEW}\`"
  echo "- Review-thread filtered: \`${LATEST_REVIEW}\`"
  if [[ "$INCLUDE_CONVERSATION" -eq 1 ]]; then
    echo "- Conversation raw: \`${RAW_CONVERSATION}\`"
    echo "- Conversation filtered: \`${LATEST_CONVERSATION}\`"
  fi
  if [[ "$INCLUDE_REVIEWS" -eq 1 ]]; then
    echo "- Review summaries raw: \`${RAW_REVIEWS}\`"
    echo "- Review summaries filtered: \`${LATEST_REVIEWS}\`"
  fi
  echo "- Combined latest: \`${LATEST_ALL}\`"
  echo "- Report: \`${REPORT}\`"
  echo "- Note: artifacts are intentionally left in \`/tmp\` for debugging; your OS typically cleans \`/tmp\` periodically."
} | tee "$REPORT"

if [[ -n "$OUT" ]]; then
  mkdir -p "$(dirname "$OUT")"
  cp "$REPORT" "$OUT"
fi
