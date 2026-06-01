<!-- SPDX-License-Identifier: Apache-2.0 OR LicenseRef-MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->

# Local verification hooks assume `.git` is a directory

Status: active bad-code note.

Legend: `PLATFORM`

## Observed failure

In the linked worktree `/Users/james/git/echo-teardown`, normal `git commit`
and `git push` both reached the meaningful docs checks, then failed while
recording local verification state:

```text
[verify-local] pre-commit: no staged Rust crates detected
mkdir: .git: Not a directory
[verify-local] failed after 0s (fresh)
```

and, after the docs-only pre-push Prettier check passed:

```text
[verify-local] pre-push docs-only change set
[verify-local] prettier --check (1 markdown files)
Checking formatting...
All matched files use Prettier code style!
mkdir: .git: Not a directory
[verify-local] failed after 0s (fresh)
```

This forced `--no-verify` for otherwise normal non-force commits and pushes
from that linked worktree.

## Root cause

In a linked Git worktree, `.git` is not a directory. It is a text file like:

```text
gitdir: /Users/james/git/echo/.git/worktrees/echo-teardown
```

The hook entrypoints are not the direct problem:

- `.githooks/pre-commit` calls `scripts/verify-local.sh pre-commit`.
- `.githooks/pre-push` calls `scripts/verify-local.sh pre-push`.
- `.githooks/_timing.sh` writes hook timing to `${repo_root}/.dx-debug`, which
  is worktree-safe.

The failure is in `scripts/verify-local.sh`:

```bash
STAMP_DIR="${VERIFY_STAMP_DIR:-.git/verify-local}"
VERIFY_TIMING_FILE="${VERIFY_TIMING_FILE:-$STAMP_DIR/timing.jsonl}"
```

Later, `write_stamp` does:

```bash
mkdir -p "$STAMP_DIR"
cat >"$path" <<EOF
```

That `mkdir -p .git/verify-local` is valid in a normal checkout, but fails in a
linked worktree because `.git` is a file. The failure happens after the actual
verification lane has already run, so successful checks can still block the
operator at the stamp-write step.

There is also an implementation/documentation mismatch: `scripts/hooks/README.md`
says timing data lands in `$(git rev-parse --git-dir)/verify-local/timing.jsonl`,
but the script currently defaults to literal `.git/verify-local`.

## Desired fix

Resolve the Git admin directory through Git instead of assuming `.git` is a
directory:

```bash
GIT_DIR="$(git rev-parse --path-format=absolute --git-dir)"
STAMP_DIR="${VERIFY_STAMP_DIR:-$GIT_DIR/verify-local}"
VERIFY_TIMING_FILE="${VERIFY_TIMING_FILE:-$STAMP_DIR/timing.jsonl}"
```

Use the same resolved path for stamp files and timing JSONL unless an explicit
`VERIFY_STAMP_DIR` or `VERIFY_TIMING_FILE` override is supplied.

## Acceptance

1. A regression test creates a temporary repository, adds a linked worktree with
   `git worktree add`, and proves `scripts/verify-local.sh pre-commit` can write
   its success stamp from inside that linked worktree.
2. A docs-only linked-worktree pre-push path can pass its docs checks and write
   its pre-push stamp without `mkdir: .git: Not a directory`.
3. Normal non-worktree checkouts still write stamps and timing data successfully.
4. `scripts/hooks/README.md` and `scripts/plot-prepush-timing.mjs` agree on the
   resolved default path, or the docs explicitly name the override behavior.

## Out of scope

- Fixing existing VitePress dead links in the docs build.
- Changing the verification lanes themselves.
- Weakening hooks or recommending `--no-verify` as normal practice.
