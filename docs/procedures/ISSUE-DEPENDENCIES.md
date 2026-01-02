<!-- SPDX-License-Identifier: Apache-2.0 OR MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->
# Procedure: GitHub Issue Dependencies (“blocked by” / “blocking”)

Echo uses **native GitHub issue dependencies** (not a custom text field) for “blocked by” relationships.

This procedure documents the **current** mechanics for viewing/adding/removing dependencies from the CLI.

## Key facts (don’t get bitten)

- The **GitHub GraphQL API currently does not provide a direct mutation** for issue dependencies.
- Use the **GitHub REST API** endpoints under `issues/{issue_number}/dependencies/...`.
- Adding/removing a dependency requires the **blocking issue’s numeric `issue_id`**, not its `#issue_number`.
- Creating dependency content too quickly can trigger **secondary rate limiting**.

## List dependencies

### List issues that `#N` is blocked by

```bash
gh api 'repos/OWNER/REPO/issues/N/dependencies/blocked_by' --jq '.[].number'
```

### List issues that `#N` is blocking

```bash
gh api 'repos/OWNER/REPO/issues/N/dependencies/blocking' --jq '.[].number'
```

## Add a “blocked by” dependency

Example: mark `#224` as blocked by `#223`.

1) Fetch the **blocking issue’s** `issue_id`:

```bash
BLOCKER_ID="$(gh api 'repos/OWNER/REPO/issues/223' --jq .id)"
echo "$BLOCKER_ID"
```

2) Add the dependency to the **blocked issue**:

```bash
gh api -X POST 'repos/OWNER/REPO/issues/224/dependencies/blocked_by' \
  -F issue_id="$BLOCKER_ID"
```

## Remove a “blocked by” dependency

The DELETE endpoint also uses the blocking issue’s **`issue_id`** (not `#`).

Example: remove the “blocked by `#223`” edge from `#224`.

```bash
BLOCKER_ID="$(gh api 'repos/OWNER/REPO/issues/223' --jq .id)"

gh api -X DELETE "repos/OWNER/REPO/issues/224/dependencies/blocked_by/${BLOCKER_ID}"
```

## Notes

- Prefer **dependency edges** over “blocked-by:” text in the issue body; the Project board can visualize dependencies directly.
- When scripting, add small sleeps/backoff if you are creating many edges to avoid secondary rate limiting.
