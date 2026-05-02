<!-- SPDX-License-Identifier: Apache-2.0 OR LicenseRef-MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->

# WIP Branch Policy

_Make intentional RED and exploratory work visible in the ref namespace, while
keeping merge-candidate branches and `main` green._

Status: Design

Owner: Echo / Platform workflow

Scope: branch policy and hook behavior design only

## Problem Statement

Echo's current local hooks treat every Rust commit as a potential green
candidate. That is correct for normal feature and review branches, but it
creates friction for METHOD RED commits.

In a strict RED/GREEN cycle, a RED commit is valuable because it captures a
failing future contract before implementation exists. Today, those commits often
require:

```sh
git commit --no-verify
```

That works, but it hides intent. `--no-verify` skips all local hook policy, not
just compile/clippy/test requirements. It also makes an intentional RED commit
look the same as a careless bypass.

The better signal is the branch ref. A branch under `wip/*` should explicitly
mean "this branch may contain intentional WIP or RED commits." Hooks can then
preserve mechanical hygiene while allowing expected compile/test failure.

## Policy Summary

Branch namespace defines workflow posture:

| Branch namespace | Meaning                              | Green required?            |
| ---------------- | ------------------------------------ | -------------------------- |
| `main`           | Shared integration branch            | Always                     |
| `feature/*`      | Merge-candidate feature work         | Yes                        |
| `review/*`       | Review, rescue, or recovery work     | Yes                        |
| `docs/*`         | Documentation work                   | Yes                        |
| `wip/*`          | RED, exploratory, backup, or handoff | No, but hygiene still runs |

`wip/*` is the only namespace allowed to contain intentional compile, clippy, or
test failures. A `wip/*` branch is not a merge candidate.

## Required Guarantees

The workflow must guarantee:

- `wip/*` may contain intentional RED/WIP commits
- `wip/*` skips only compile, clippy, and test green requirements
- `wip/*` still enforces formatting, SPDX, markdown, lockfile, and cheap hygiene
- `wip/*` may push only to `refs/heads/wip/*`
- `wip/*` must never push to `main`, `feature/*`, `review/*`, or `docs/*`
- merge-candidate work must happen on a separate `feature/*` branch created
  from the WIP branch after GREEN begins or passes
- branch rename is not the preferred promotion workflow
- `main` remains green-only

## Human User Stories

- As a runtime maintainer, I want RED commits to be visible as WIP by branch
  name so that failing tests are intentional rather than hidden behind
  `--no-verify`.
- As a reviewer, I want `feature/*` and `review/*` branches to keep normal green
  requirements so that merge candidates do not inherit WIP leniency.
- As a maintainer, I want push guards that prevent accidental WIP-to-feature or
  WIP-to-main ref updates.
- As a contributor, I want WIP commits to keep formatting and documentation
  hygiene so that failed compile does not become general sloppiness.

## Agent User Stories

- As a coding agent, I want to commit RED tests on `wip/*` without bypassing all
  hooks so that METHOD history is durable and explicit.
- As a review agent, I want branch namespace to tell me whether failing tests are
  expected WIP or a merge blocker.
- As a workflow agent, I want pre-push guards that reject dangerous refspecs such
  as `wip/foo:main` or `wip/foo:feature/foo`.
- As a future hook-implementation agent, I want the policy split designed before
  editing hooks so that the implementation stays surgical.

## Hills

### Hill 1: RED Commits Without `--no-verify`

- **Who:** Coding agents and runtime maintainers.
- **What:** Commit intentional RED tests on `wip/*` while preserving formatting,
  SPDX, markdown, lockfile, and cheap hygiene checks.
- **Wow:** RED commits become normal commits with explicit branch-level intent
  instead of blanket hook bypasses.

### Hill 2: Merge Candidates Stay Green

- **Who:** Reviewers and maintainers.
- **What:** Keep `main`, `feature/*`, `review/*`, and `docs/*` under normal
  green requirements.
- **Wow:** WIP leniency cannot leak into a branch that is eligible for PR/merge.

### Hill 3: Dangerous Ref Pushes Are Blocked

- **Who:** Maintainers.
- **What:** Reject pushes from `wip/*` to non-WIP remote refs, especially
  `refs/heads/main`, `refs/heads/feature/*`, `refs/heads/review/*`, and
  `refs/heads/docs/*`.
- **Wow:** A fat-fingered refspec cannot promote failing WIP into a protected or
  merge-candidate namespace.

## Non-Goals

This design does not implement hooks yet.

It also does not:

- weaken `main` or PR branch protection
- allow WIP branches to merge
- remove formatting, SPDX, markdown, lockfile, or cheap hygiene checks
- add force-push workflows
- permit rebasing or commit amendment
- redesign CI
- replace METHOD
- resume docs inventory

## Branch Lifecycle

### Start RED or Exploratory Work

Start WIP work from current `main`:

```sh
git switch main
git pull origin main
git switch -c wip/foo-red
```

`wip/foo-red` may contain intentionally failing RED tests or exploratory work.
It may be pushed only to:

```text
refs/heads/wip/foo-red
```

### Start GREEN or Merge-Candidate Work

When GREEN begins or passes, create a separate merge-candidate branch from the
WIP branch:

```sh
git switch wip/foo-red
git switch -c feature/foo
```

Do not prefer renaming:

```sh
git branch -m feature/foo
```

Branching preserves the mental model:

```text
wip/foo-red = may fail, backup/review/handoff
feature/foo = merge candidate, must be green
main        = shared green integration branch
```

The `feature/*` branch must pass normal hooks before push or PR.

## Pre-Commit Policy

Pre-commit behavior should be branch-aware.

On all branches, including `wip/*`, pre-commit still enforces:

- toolchain pin
- rustfmt / Prettier formatting
- markdownlint for staged Markdown
- SPDX headers
- lockfile format
- PRNG coupling guard
- task-list guard

On non-WIP branches, pre-commit also runs the normal staged Rust verification:

```sh
scripts/verify-local.sh pre-commit
```

On `wip/*`, pre-commit may skip only the staged Rust compile/clippy green gate.
The hook should print an explicit warning:

```text
pre-commit: WIP branch 'wip/foo-red' detected.
pre-commit: skipping compile/clippy pre-commit verification only.
pre-commit: this branch is not a merge candidate.
```

## Pre-Push Policy

Pre-push must inspect the pushed ref updates from stdin. Policy must be based on
the local and remote refs involved in the push, not only on the current checked
out branch.

Allowed WIP push:

```text
refs/heads/wip/foo-red -> refs/heads/wip/foo-red
```

Rejected pushes:

```text
refs/heads/wip/foo-red -> refs/heads/main
refs/heads/wip/foo-red -> refs/heads/feature/foo
refs/heads/wip/foo-red -> refs/heads/review/foo
refs/heads/wip/foo-red -> refs/heads/docs/foo
refs/heads/wip/foo-red -> refs/heads/foo
```

If every pushed destination is under `refs/heads/wip/*`, pre-push may run a
reduced WIP push gate:

- format check
- Markdown formatting/linting for changed Markdown
- SPDX check
- lockfile format check
- hook syntax checks if hook files changed

If any pushed destination is outside `refs/heads/wip/*`, normal pre-push
verification applies. If a local `wip/*` ref targets a non-WIP remote ref, the
push must fail before running expensive verification.

## CI Policy

`wip/*` branches should not pretend to be merge candidates.

Preferred CI behavior for pushed WIP branches:

- do not run the full merge-candidate matrix
- run only a WIP hygiene workflow if CI is configured for WIP refs
- avoid noisy red CI caused by intentional RED compile failures

Full CI remains required for:

- `main`
- PRs to `main`
- merge-candidate branch pushes where configured

## Test Plan

### Golden Tests

- pre-commit on `wip/foo-red` runs formatting and markdown/SPDX hygiene
- pre-commit on `wip/foo-red` skips only staged Rust compile/clippy verification
- pre-commit on `feature/foo` runs normal staged Rust verification
- pre-push allows `refs/heads/wip/foo-red -> refs/heads/wip/foo-red`
- pre-push runs normal verification for `feature/foo`

### Known Failure Tests

- pre-push rejects `refs/heads/wip/foo-red -> refs/heads/main`
- pre-push rejects `refs/heads/wip/foo-red -> refs/heads/feature/foo`
- pre-push rejects `refs/heads/wip/foo-red -> refs/heads/review/foo`
- pre-push rejects `refs/heads/wip/foo-red -> refs/heads/docs/foo`
- pre-commit on `feature/foo` fails if staged Rust compile/check fails
- pre-commit on `wip/foo-red` still fails if formatting or SPDX checks fail

### Edge Tests

- detached HEAD falls back to normal verification
- empty branch name falls back to normal verification
- multiple pre-push ref updates with one non-WIP destination run or require
  normal policy
- deleted remote refs do not accidentally bypass policy
- pushing tags is not treated as a WIP branch bypass
- local branch named `wip` without slash is not treated as WIP

### Non-Goal Guard Tests

- no hook path disables all checks for `wip/*`
- no hook path allows WIP-to-main or WIP-to-feature pushes
- no hook path permits force push
- no workflow doc recommends branch rename as the preferred promotion path
- no workflow doc describes WIP branches as PR-ready

## Implementation Plan

### RED 1

Hook-policy tests fail because branch-aware WIP behavior does not exist.

Expected failing tests:

- `pre_commit_wip_branch_skips_only_compile_gate`
- `pre_commit_feature_branch_runs_compile_gate`
- `pre_push_allows_wip_to_wip`
- `pre_push_rejects_wip_to_main`
- `pre_push_rejects_wip_to_feature`

### GREEN 1

Add branch/ref parsing helpers.

Expected implementation:

- parse current branch in pre-commit
- parse local/remote refs from pre-push stdin
- classify `refs/heads/wip/*` exactly
- treat `wip` without slash as non-WIP

### GREEN 2

Make pre-commit WIP-aware.

Expected implementation:

- keep formatting, SPDX, lockfile, markdown, and cheap guards always on
- skip only `scripts/verify-local.sh pre-commit` on `wip/*`
- print an explicit WIP warning

### GREEN 3

Make pre-push ref-aware.

Expected implementation:

- reject local `wip/*` refs targeting non-WIP remote refs
- allow WIP-to-WIP pushes with reduced hygiene
- run normal pre-push verification for non-WIP destinations

### GREEN 4

Document the workflow in the contributor workflow docs.

Expected implementation:

- add a short `wip/*` workflow section to `docs/workflows.md`
- update the RED/GREEN friction note from open problem to resolved policy

## Playback After GREEN

- [ ] RED commits can be committed on `wip/*` without `--no-verify`.
- [ ] WIP commits still enforce formatting, SPDX, markdown, lockfile, and cheap
      hygiene.
- [ ] WIP branches can push only to `refs/heads/wip/*`.
- [ ] WIP-to-main and WIP-to-feature pushes are rejected.
- [ ] Merge-candidate branches still run normal gates.
- [ ] The documented promotion workflow uses `git switch -c feature/foo`, not
      branch rename.
- [ ] No hook implementation disables all checks for WIP.

## Verification Commands

Design-only gate for this document:

```sh
pnpm exec prettier --check docs/design/wip-branch-policy.md
pnpm exec markdownlint-cli2 docs/design/wip-branch-policy.md
pnpm docs:build
```

Expected implementation gates for a later hook cycle:

```sh
cargo fmt --all -- --check
cargo test -p xtask --lib hook
bash -n .githooks/pre-commit .githooks/pre-push scripts/verify-local.sh
pnpm docs:build
```
