<!-- SPDX-License-Identifier: Apache-2.0 OR MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->
# Procedure: Extract Actionable Comments from PR Review Threads (CodeRabbitAI + Humans)

This procedure is part of the required PR workflow for this repo.

GitHub carries forward review comments across commits, so you must extract only the **currently actionable** feedback (not already-fixed or stale comments) before starting another fix batch.

---

## Expected Workflow Context (Where This Fits)

When you finish work:

1. Push a branch and open a PR.
2. Wait for CI + CodeRabbitAI.
3. Extract actionable comments (this doc).
4. Fix issues in small commits + push.
5. Repeat until CodeRabbitAI (and any required human reviewer) approves.

---

## Prerequisites

- `gh` installed and authenticated
- `jq` installed (minimum: `jq >= 1.6`)
- Repo access to view PRs

---

## Procedure

### Quick Recommendation

Prefer the repo automation whenever possible:

```bash
.github/scripts/extract-actionable-comments.sh <PR_NUMBER> --full
```

It is designed to:

- include review comments from **all** authors (CodeRabbitAI *and* human reviewers),
- include “outdated” comments that are not visible on the current head diff,
- detect `✅ Addressed in commit ...` markers in replies (authored by a human),
- and produce a deterministic Markdown report.

To widen the net beyond inline review threads, you can include:

- PR conversation comments (top-level PR timeline discussion), and
- review summaries (approve / changes-requested review bodies).

```bash
.github/scripts/extract-actionable-comments.sh <PR_NUMBER> --all-sources --full
```

Note:
- Conversation comments and review summaries are not diff-positioned like review threads, so the script applies a simple “likely actionable” heuristic and emits a separate “Unclassified” bucket for anything that doesn’t match.

### Step 1: Identify the PR head commit (the current diff)

```bash
PR_NUMBER="<PR_NUMBER>"
LATEST_COMMIT="$(gh pr view "$PR_NUMBER" --json headRefOid --jq '.headRefOid[0:7]')"
echo "PR head commit: $LATEST_COMMIT"
```

Why: comment staleness is measured relative to the current PR head.

---

### Step 2: Fetch all review comments (PR review threads)

```bash
OWNER="<OWNER>"
REPO="<REPO>"
TMPFILE="/tmp/pr-${PR_NUMBER}-comments-$(date +%s).json"
gh api "repos/${OWNER}/${REPO}/pulls/${PR_NUMBER}/comments" --paginate > "$TMPFILE"
```

Note:
- This endpoint returns PR review comments authored by anyone (humans, bots, CodeRabbitAI, etc.).

---

### Step 3: Extract top-level review comments (including “outdated”)

Important:

- GitHub’s review comments API (`/pulls/:number/comments`) keeps each comment’s `commit_id` fixed to the commit it was authored on.
- When the PR head moves, older unresolved comments usually become **outdated** rather than being re-bound to the new head.
- If you filter only to `commit_id == PR_HEAD`, you can incorrectly report “0 actionables” while older threads remain open.

```bash
cat "$TMPFILE" | jq --arg head "$LATEST_COMMIT" '
  .[] |
  select(.in_reply_to_id == null) |
  {
    id,
    line,
    path,
    position,
    head_commit: $head,
    comment_commit: .commit_id[0:7],
    original_commit: .original_commit_id[0:7],
    is_visible_on_head_diff: (.position != null),
    is_outdated: (.position == null),
    is_moved: (.commit_id != .original_commit_id),
    created_at,
    body_preview: (.body[0:200])
  }
' | jq -s '.' > /tmp/comments-latest.json
```

---

### Step 4: Bucket on-head vs outdated (and verify against current code)

```bash
cat /tmp/comments-latest.json | jq '
  group_by(.is_outdated) |
  map({
    category: (if .[0].is_outdated then "OUTDATED (earlier commit)" else "ON_HEAD" end),
    count: length,
    comments: map({id, line, path, position, comment_commit, original_commit})
  })
'
```

Key insight:
- “Outdated” means “not visible on the current head diff”, **not** “fixed”.
- Always verify against current code before acting (see Step 7).

---

### Step 5: Detect “Already Addressed” markers

The “✅ Addressed in commit …” marker is a lightweight ack convention used to prevent re-triaging the same comments.

Important: **do not** treat a bare substring match as reliable.

- Review bots (including CodeRabbitAI) may include the exact string as a template/example in their own review text.
- If you count those as “acknowledged”, you can incorrectly report “0 actionables” and miss real work.

For **review threads**, prefer a human-authored reply containing a commit SHA that is actually part of the PR.

For **PR conversation comments** and **review summaries**, you may also use the marker in your own comment body (or edit), but only treat it as acknowledged when the marker includes a real PR commit SHA.

If you want reliable ack detection, prefer the repo script:

```bash
.github/scripts/extract-actionable-comments.sh <PR_NUMBER>
```

```bash
cat "$TMPFILE" | jq '
  # Very rough, but safer than substring matching:
  # - only count replies (in_reply_to_id != null)
  # - only count markers that start a line and include a hex SHA
  [ .[]
    | select(.in_reply_to_id != null)
    | select(.body | test("(?m)^\\s*✅ Addressed in commit [0-9a-f]{7,40}\\b"))
    | { in_reply_to_id, reply_id: .id, user: (.user.login // "unknown"), body: (.body[0:80]) }
  ]
'
```

Key insight:
- Explicit acks are only useful when they can’t be accidentally “spoofed” by templated bot text.

---

### Step 6: Categorize by priority (optional)

This is only useful if CodeRabbitAI uses explicit priority markers in comment bodies.

```bash
cat "$TMPFILE" | jq --arg head "$LATEST_COMMIT" '
  .[] |
  select(
    .in_reply_to_id == null
  ) |
  {
    id,
    line,
    path,
    priority: (
      if (.body | test("\\bP0\\b|badge/P0-|🔴|Critical"; "i")) then "P0"
      elif (.body | test("\\bP1\\b|badge/P1-|🟠|Major"; "i")) then "P1"
      elif (.body | test("\\bP2\\b|badge/P2-|🟡|Minor"; "i")) then "P2"
      else "P3"
      end
    ),
    is_visible_on_head_diff: (.position != null),
    is_outdated: (.position == null),
    body
  }
' | jq -s '.' > /tmp/prioritized-comments.json
```

---

### Step 7: Verify outdated comments against current code (critical step)

Do not trust `is_outdated` alone. Verify by mapping fields from the comment object to concrete commands.

Suggested mapping:

- `path` → file path
- `line` → line number (use a small context window, e.g. ±5 lines)

Example:

```bash
# Suppose you have a single comment object (e.g., from /tmp/comments-latest.json):
COMMENT_PATH="docs/decision-log.md"
COMMENT_LINE=42

# Clamp the start line to 1 (sed doesn't like 0/negative ranges).
START=$((COMMENT_LINE - 5))
if [[ "$START" -lt 1 ]]; then START=1; fi
END=$((COMMENT_LINE + 5))

# 1) Inspect current state around the line
git show "HEAD:${COMMENT_PATH}" | sed -n "${START},${END}p"

# 2) Scan recent history for related fixes
git log --all --oneline -- "${COMMENT_PATH}" | head -20

# 3) If the comment mentions a specific identifier (function/struct name), search by token
git log -p --all -S"SomeIdentifierFromComment" -- "${COMMENT_PATH}" | head -80
```

If the comment is outdated (not visible on head diff), it may refer to old line numbers. In that case:

- search by keyword/token rather than trusting the line number, and
- look up the original code context via `original_commit_id` if needed.

```bash
# Use the repo script if you want the comment bodies included:
.github/scripts/extract-actionable-comments.sh <PR_NUMBER> --full
```

---

### Step 8: Produce an issue report (batch)

Create a batch checklist and work top-down:

```bash
cat > /tmp/batch-N-issues.md << 'EOF'
# Batch N - CodeRabbitAI Issues

## Outdated (Verify / Already Fixed)
- [ ] Line XXX - Issue description (Fixed in: COMMIT_SHA)

## P0 Critical
- [ ] Line XXX - Issue description

## P1 Major
- [ ] Line XXX - Issue description

## P2 Minor
- [ ] Line XXX - Issue description

## P3 Trivial
- [ ] Line XXX - Issue description
EOF
```

---

### Step 9: Save full bodies for needs-attention issues

Prefer the helper script, which understands ack markers in replies and can print full bodies:

```bash
.github/scripts/extract-actionable-comments.sh <PR_NUMBER> --full
```

---

## When CodeRabbitAI approval doesn’t unblock GitHub

If CodeRabbitAI approved but GitHub still shows “changes requested”, nudge the bot:

```text
@coderabbitai Please review the latest commit and clear the "changes requested" status since you have already approved the changes.
```

---

## Automation

Use the helper script:

- `.github/scripts/extract-actionable-comments.sh`
