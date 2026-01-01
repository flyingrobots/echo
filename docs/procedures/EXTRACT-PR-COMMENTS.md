<!-- SPDX-License-Identifier: Apache-2.0 OR MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->
# Procedure: Extract Actionable Comments from CodeRabbitAI PR Reviews

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
- `jq` installed
- Repo access to view PRs

---

## Procedure

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

---

### Step 3: Extract top-level comments pinned to the latest commit

```bash
cat "$TMPFILE" | jq --arg commit "$LATEST_COMMIT" '
  .[] |
  select(.in_reply_to_id == null and .commit_id[0:7] == $commit) |
  {
    id,
    line,
    path,
    current_commit: .commit_id[0:7],
    original_commit: .original_commit_id[0:7],
    is_stale: (.commit_id != .original_commit_id),
    created_at,
    body_preview: (.body[0:200])
  }
' | jq -s '.' > /tmp/comments-latest.json
```

---

### Step 4: Identify stale vs fresh comments

```bash
cat /tmp/comments-latest.json | jq '
  group_by(.is_stale) |
  map({
    category: (if .[0].is_stale then "STALE" else "FRESH" end),
    count: length,
    comments: map({id, line, path, original_commit})
  })
'
```

Key insight:
- If `is_stale == true`, the comment originated on an earlier commit and may already be fixed.

---

### Step 5: Detect “Already Addressed” markers

```bash
cat "$TMPFILE" | jq '.[] |
  select(.body | contains("✅ Addressed in commit")) |
  {
    id,
    line,
    path,
    fixed_in: (.body | capture("✅ Addressed in commit (?<commit>[a-f0-9]{7})").commit)
  }
'
```

Key insight:
- If the comment contains a “✅ Addressed in commit …” marker, it’s no longer actionable.

---

### Step 6: Categorize by priority (optional)

This is only useful if CodeRabbitAI uses explicit priority markers in comment bodies.

```bash
cat "$TMPFILE" | jq --arg commit "$LATEST_COMMIT" '
  .[] |
  select(
    .in_reply_to_id == null and
    .commit_id[0:7] == $commit
  ) |
  {
    id,
    line,
    path,
    priority: (
      if (.body | contains("🔴 Critical")) then "P0"
      elif (.body | contains("🟠 Major")) then "P1"
      elif (.body | contains("🟡 Minor")) then "P2"
      else "P3"
      end
    ),
    is_stale: (.commit_id != .original_commit_id),
    body
  }
' | jq -s '.' > /tmp/prioritized-comments.json
```

---

### Step 7: Verify stale comments against current code (critical step)

Do not trust `is_stale` alone. Verify:

```bash
# 1) Inspect current state
git show "HEAD:<file_path>" | sed -n '<start>,<end>p'

# 2) Search history for fixes (if needed)
git log --all --oneline --grep="<keyword>"
git log -p --all -S"<code_pattern>" -- <file_path>
```

---

### Step 8: Produce an issue report (batch)

Create a batch checklist and work top-down:

```bash
cat > /tmp/batch-N-issues.md << 'EOF'
# Batch N - CodeRabbitAI Issues

## Stale (Verify / Already Fixed)
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

### Step 9: Save full bodies for actionable issues

```bash
cat /tmp/prioritized-comments.json | jq -r '.[] |
  select(.is_stale == false) |
  "# Comment ID: \(.id) - Line \(.line)\n\(.body)\n\n---\n\n"
' > /tmp/batch-N-full-comments.txt
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

