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

REPO=""
OUT=""
FULL=0

while [[ $# -gt 0 ]]; do
  case "$1" in
    --repo)
      REPO="${2:-}"
      shift 2
      ;;
    --out)
      OUT="${2:-}"
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

HEAD_SHA="$(gh pr view "$PR_NUMBER" --repo "$REPO" --json headRefOid --jq '.headRefOid')"
HEAD7="${HEAD_SHA:0:7}"

TS="$(date +%s)"
RAW="/tmp/pr-${PR_NUMBER}-comments-${TS}.json"
LATEST="/tmp/pr-${PR_NUMBER}-latest-${TS}.json"
REPORT="/tmp/pr-${PR_NUMBER}-report-${TS}.md"

gh api "repos/${OWNER}/${NAME}/pulls/${PR_NUMBER}/comments" --paginate > "$RAW"

# Filter: top-level comments pinned to the current PR head commit.
jq --arg commit "$HEAD7" '
  [ .[] |
    select(.in_reply_to_id == null and .commit_id[0:7] == $commit) |
    {
      id,
      path,
      line,
      current_commit: (.commit_id[0:7]),
      original_commit: (.original_commit_id[0:7]),
      is_stale: (.commit_id != .original_commit_id),
      has_ack: (.body | contains("✅ Addressed in commit")),
      priority: (
        if (.body | contains("🔴 Critical")) then "P0"
        elif (.body | contains("🟠 Major")) then "P1"
        elif (.body | contains("🟡 Minor")) then "P2"
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

fresh_count="$(jq '[.[] | select(.is_stale == false and .has_ack == false)] | length' "$LATEST")"
stale_count="$(jq '[.[] | select(.is_stale == true)] | length' "$LATEST")"
ack_count="$(jq '[.[] | select(.has_ack == true)] | length' "$LATEST")"

{
  echo "# CodeRabbitAI/GitHub Actionables — PR #${PR_NUMBER}"
  echo
  echo "- Repo: \`${REPO}\`"
  echo "- PR head: \`${HEAD7}\`"
  echo "- Generated: \`$(date -u +"%Y-%m-%dT%H:%M:%SZ")\`"
  echo

  echo "## Summary"
  echo
  echo "- Fresh actionable: **${fresh_count}**"
  echo "- Stale (verify): **${stale_count}**"
  echo "- Acknowledged (✅ Addressed): **${ack_count}**"
  echo

  echo "## Stale / Verify"
  echo
  jq -r '
    .[]
    | select(.is_stale == true)
    | "- [ ] \(.path):\(.line // 1) — \(.title) (original: \(.original_commit)) [id=\(.id)]"
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

  echo "## Actionable (Fresh)"
  echo

  # Sort: priority, then path, then line (null line => 0).
  jq -r '
    def pnum(p):
      if p == "P0" then 0
      elif p == "P1" then 1
      elif p == "P2" then 2
      else 3
      end;

    [ .[]
      | select(.is_stale == false and .has_ack == false)
    ]
    | sort_by([pnum(.priority), .path, (.line // 0)])
    | .[]
    | "- [ ] [\(.priority)] \(.path):\(.line // 1) — \(.title) [id=\(.id)]"
  ' "$LATEST"
  echo

  if [[ "$FULL" -eq 1 ]]; then
    echo "## Full Comment Bodies (Actionable)"
    echo
    jq -r '
      def pnum(p):
        if p == "P0" then 0
        elif p == "P1" then 1
        elif p == "P2" then 2
        else 3
        end;

      [ .[]
        | select(.is_stale == false and .has_ack == false)
      ]
      | sort_by([pnum(.priority), .path, (.line // 0)])
      | .[]
      | "### \(.path):\(.line // 1) [id=\(.id)]\n\n```\n\(.body)\n```\n"
    ' "$LATEST"
  else
    echo "## Next Step"
    echo
    echo "Run with \`--full\` to include full comment bodies for the actionable set."
  fi

  echo
  echo "## Artifacts"
  echo
  echo "- Raw comments: \`${RAW}\`"
  echo "- Filtered latest: \`${LATEST}\`"
  echo "- Report: \`${REPORT}\`"
} | tee "$REPORT"

if [[ -n "$OUT" ]]; then
  mkdir -p "$(dirname "$OUT")"
  cp "$REPORT" "$OUT"
fi

