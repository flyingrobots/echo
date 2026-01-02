#!/usr/bin/env bash
# SPDX-License-Identifier: Apache-2.0
# © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>

set -euo pipefail

usage() {
  cat <<'EOF'
Usage:
  .github/scripts/extract-actionable-comments.sh <PR_NUMBER> [--repo OWNER/REPO] [--out <path>] [--full]

Purpose:
  Extract actionable (fresh) CodeRabbitAI/GitHub review comments for a PR by
  filtering comments pinned to the PR head commit and grouping by staleness.

Outputs:
  - Writes a Markdown report to stdout by default.
  - Also writes the raw comments JSON and intermediate files to /tmp.

Options:
  --repo OWNER/REPO   Override repo (default: current repo via `gh repo view`)
  --out <path>        Write the Markdown report to <path> (also prints to stdout)
  --full              Include full comment bodies for actionable comments

Examples:
  .github/scripts/extract-actionable-comments.sh 176
  .github/scripts/extract-actionable-comments.sh 176 --out /tmp/pr-176-report.md
  .github/scripts/extract-actionable-comments.sh 176 --full
EOF
}

require_cmd() {
  local cmd="$1"
  command -v "$cmd" >/dev/null 2>&1 || { echo "Missing dependency: $cmd" >&2; exit 2; }
}

PR_NUMBER="${1:-}"
shift || true
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
HEAD7="${HEAD_SHA:0:7}"

TS="$(date +%s)"
RAW="/tmp/pr-${PR_NUMBER}-comments-${TS}.json"
RAW_ERR="/tmp/pr-${PR_NUMBER}-comments-${TS}.err"
LATEST="/tmp/pr-${PR_NUMBER}-latest-${TS}.json"
REPORT="/tmp/pr-${PR_NUMBER}-report-${TS}.md"

attempt=1
delay_s=1
while true; do
  if gh api "repos/${OWNER}/${NAME}/pulls/${PR_NUMBER}/comments" --paginate > "$RAW" 2> "$RAW_ERR"; then
    break
  fi

  if [[ "$attempt" -ge 4 ]]; then
    echo "Error: Failed to fetch PR comments from GitHub after ${attempt} attempts." >&2
    echo "Repo: ${REPO}" >&2
    echo "PR: ${PR_NUMBER}" >&2
    echo >&2
    echo "Troubleshooting:" >&2
    echo "- Run: gh auth status" >&2
    echo "- Check rate limits / token scopes (GH_TOKEN) and retry later" >&2
    echo "- Verify repo/PR access permissions" >&2
    echo "- Check network connectivity" >&2
    echo >&2
    echo "gh api stderr (last attempt):" >&2
    sed -n '1,200p' "$RAW_ERR" >&2
    exit 1
  fi

  sleep "$delay_s"
  delay_s="$((delay_s * 2))"
  attempt="$((attempt + 1))"
done

if ! jq -e . "$RAW" >/dev/null 2>&1; then
  echo "Error: GitHub API returned invalid JSON in: ${RAW}" >&2
  echo "gh api stderr:" >&2
  sed -n '1,200p' "$RAW_ERR" >&2
  exit 1
fi

# Collect: all top-level review comments (including ones authored on earlier commits).
#
# Why: the PR review comments API (`/pulls/:number/comments`) keeps each comment’s
# `commit_id` fixed to the commit it was authored on. When new commits are pushed,
# older unresolved comments do not “move” to the new head; they become “outdated”.
# If we only include comments whose `commit_id` matches the current head, we can
# incorrectly report “0 actionables” even though older review threads remain open.
jq --arg head "$HEAD7" '
  def has_ack_marker(s):
    (s | type) == "string" and (s | contains("✅ Addressed in commit"));

  # Replies are returned in the same list, with `in_reply_to_id` set.
  # Treat a top-level comment as "acknowledged" if either:
  # - the comment body itself contains the marker, OR
  # - any reply to it contains the marker.
  def ack_by_reply:
    reduce .[] as $c ({}; if ($c.in_reply_to_id != null and has_ack_marker($c.body)) then .[($c.in_reply_to_id | tostring)] = true else . end);

  (ack_by_reply) as $replies |
  [ .[] |
    select(.in_reply_to_id == null) |
    {
      id,
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
      has_ack: (has_ack_marker(.body) or ($replies[(.id | tostring)] // false)),
      priority: (
        if (.body | test("\\\\bP0\\\\b|badge/P0-|🔴|Critical"; "i")) then "P0"
        elif (.body | test("\\\\bP1\\\\b|badge/P1-|🟠|Major"; "i")) then "P1"
        elif (.body | test("\\\\bP2\\\\b|badge/P2-|🟡|Minor"; "i")) then "P2"
        else "P3"
        end
      ),
      title: (
        (.body
          | split("\n")
          | map(select(. != ""))
          | .[0] // "UNTITLED"
        )
        | gsub("\\*\\*"; "")
        | .[0:80]
      ),
      body: .body
    }
  ]
' "$RAW" > "$LATEST"

needs_attention_count="$(jq '[.[] | select(.has_ack == false)] | length' "$LATEST")"
on_head_attention_count="$(jq '[.[] | select(.is_visible_on_head_diff == true and .has_ack == false)] | length' "$LATEST")"
outdated_attention_count="$(jq '[.[] | select(.is_outdated == true and .has_ack == false)] | length' "$LATEST")"
moved_count="$(jq '[.[] | select(.is_moved == true)] | length' "$LATEST")"
ack_count="$(jq '[.[] | select(.has_ack == true)] | length' "$LATEST")"
total_count="$(jq 'length' "$LATEST")"

{
  echo "# CodeRabbitAI/GitHub Actionables — PR #${PR_NUMBER}"
  echo
  echo "- Repo: \`${REPO}\`"
  echo "- PR head: \`${HEAD7}\`"
  echo "- Generated: \`$(date -u +"%Y-%m-%dT%H:%M:%SZ")\`"
  echo

  echo "## Summary"
  echo
  echo "- Total top-level review comments: **${total_count}**"
  echo "- Needs attention (unacknowledged): **${needs_attention_count}**"
  echo "  - Visible on head diff: **${on_head_attention_count}**"
  echo "  - Outdated (not visible on head diff): **${outdated_attention_count}**"
  echo "- Acknowledged (✅ Addressed): **${ack_count}**"
  echo "- Moved by GitHub (commit_id != original_commit_id): **${moved_count}**"
  echo

  echo "## Needs Attention (On Head Diff)"
  echo
  jq -r '
    .[]
    | select(.is_visible_on_head_diff == true and .has_ack == false)
    | "- [ ] [\(.priority)] \(.path):\(.line // 1) — \(.title) [id=\(.id)]"
  ' "$LATEST"
  echo

  echo "## Needs Attention (Outdated / Earlier Commits)"
  echo
  jq -r '
    .[]
    | select(.is_outdated == true and .has_ack == false)
    | "- [ ] [\(.priority)] \(.path):\(.line // 1) — \(.title) (comment commit: \(.comment_commit)) [id=\(.id)]"
  ' "$LATEST"
  echo

  echo "## Acknowledged"
  echo
  jq -r '
    .[]
    | select(.has_ack == true)
    | "- [ ] \(.path):\(.line // 1) — \(.title) (acknowledged) [id=\(.id)]"
  ' "$LATEST"
  echo

  echo "## Notes"
  echo
  echo "- \"Outdated\" means the review comment is no longer visible on the current head diff; it may still be actionable."
  echo "- Use \`✅ Addressed in commit <sha>\` replies to close the loop and keep future extraction cheap."
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
        | select(.has_ack == false)
      ]
      | sort_by([(.is_outdated | if . then 1 else 0 end), pnum(.priority), .path, (.line // 0)])
      | .[]
      | "### [\(.priority)] \(.path):\(.line // 1) [id=\(.id)]\n\n- Visible on head diff: \(.is_visible_on_head_diff)\n- Outdated: \(.is_outdated)\n- Comment commit: \(.comment_commit)\n\n````\n\(.body)\n````\n"
    ' "$LATEST"
  else
    echo "## Next Step"
    echo
    echo "Run with \`--full\` to include full comment bodies for the needs-attention set."
  fi

  echo
  echo "## Artifacts"
  echo
  echo "- Raw comments: \`${RAW}\`"
  echo "- Filtered latest: \`${LATEST}\`"
  echo "- Report: \`${REPORT}\`"
  echo "- Note: artifacts are intentionally left in \`/tmp\` for debugging; your OS typically cleans \`/tmp\` periodically."
} | tee "$REPORT"

if [[ -n "$OUT" ]]; then
  mkdir -p "$(dirname "$OUT")"
  cp "$REPORT" "$OUT"
fi
