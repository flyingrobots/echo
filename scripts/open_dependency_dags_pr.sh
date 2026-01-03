#!/usr/bin/env bash
# SPDX-License-Identifier: Apache-2.0
# © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>

set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"

usage() {
  cat <<'USAGE'
Usage: scripts/open_dependency_dags_pr.sh [--issue <number>] [--base <branch>] [--head <branch>]

Creates (or reuses) an issue, pushes the current branch, and opens a PR that closes the issue.

Options:
  --issue <number>   Use an existing issue number instead of creating a new issue.
  --base <branch>    PR base branch (default: main).
  --head <branch>    PR head branch (default: current branch).

Notes:
  - Requires `gh` authentication and `git push` access.
  - Does not rebase or force-push; pushes the branch as-is.
USAGE
}

issue_number=""
base_branch="main"
head_branch="$(git branch --show-current)"

while [[ $# -gt 0 ]]; do
  case "$1" in
    --issue)
      issue_number="${2:-}"
      shift 2
      ;;
    --base)
      base_branch="${2:-}"
      shift 2
      ;;
    --head)
      head_branch="${2:-}"
      shift 2
      ;;
    -h|--help)
      usage
      exit 0
      ;;
    *)
      echo "Unknown arg: $1" >&2
      usage >&2
      exit 2
      ;;
  esac
done

if [[ -z "$head_branch" ]]; then
  echo "Error: could not determine current branch." >&2
  exit 1
fi

if ! command -v gh >/dev/null 2>&1; then
  echo "Error: gh not found (install GitHub CLI)." >&2
  exit 1
fi

if ! gh auth status >/dev/null 2>&1; then
  echo "Error: gh is not authenticated. Run: gh auth login" >&2
  exit 1
fi

if ! git rev-parse --is-inside-work-tree >/dev/null 2>&1; then
  echo "Error: must run inside a git work tree." >&2
  exit 1
fi

if [[ -n "$(git status --porcelain)" ]]; then
  echo "Error: working tree is dirty. Commit or stash before opening the PR." >&2
  git status --porcelain >&2
  exit 1
fi

if [[ -z "$issue_number" ]]; then
  title="Automate dependency DAG refresh + document contributor workflows"
  body=$'Tracks dependency DAG generation + automation + contributor workflow docs.\n\nAcceptance:\n- `cargo xtask dags --snapshot-label none` regenerates deterministic DOT/SVG\n- Scheduled workflow opens PR only when `docs/assets/dags/*` changes\n- README links to workflow docs\n'

  issue_url="$(gh issue create --title "$title" --body "$body")"
  issue_number="$(gh issue view "$issue_url" --json number -q .number)"
  echo "Created issue #$issue_number ($issue_url)"
else
  echo "Using existing issue #$issue_number"
fi

echo "Pushing branch: $head_branch"
git push -u origin "$head_branch"

pr_title="docs: dependency DAG automation + workflows (#${issue_number})"
pr_body=$'Implements dependency DAG generation + automation, and documents contributor workflows.\n\nCloses #'"${issue_number}"$'\n\nTest Plan:\n- cargo xtask dags --snapshot-label none\n- cargo xtask dags --snapshot-label rolling --no-render\n'

echo "Opening PR: $head_branch -> $base_branch"
gh pr create --base "$base_branch" --head "$head_branch" --title "$pr_title" --body "$pr_body"
