<!-- SPDX-License-Identifier: Apache-2.0 OR LicenseRef-MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->

# Doghouse

The Doghouse is the design bay for the PR Flight Recorder.

This directory exists so the recorder does not become "whatever seemed easy to code next."
The first implementation slice is already real in `cargo xtask pr-snapshot`, but the next
slices need product discipline before they need more code.

## Why This Exists

Long-lived PRs with multiple pushes, rerun checks, and automated reviewers become hard to
reason about. The author stops trusting memory, GitHub mixes historical and live state, and
the CLI is not much better. The recorder's job is to restore clarity.

The product question is not "can we diff two JSON blobs?"

The product question is:

- what changed since the last sortie
- what matters now
- what action should happen next

## Working Principle

- Capture trustworthy local state first.
- Interpret state in terms of blockers, not raw fields.
- Prefer semantic deltas over raw file diffs.
- Preserve the Draft Punks flavor later, but earn it with a solid mechanic first.

## Current Plumbing

The agent-native plumbing entrypoint is:

```sh
cargo xtask doghouse sortie 308
```

That command emits JSONL to stdout, writes local snapshot/delta artifacts under
`artifacts/pr-review/`, and includes a machine-usable next-action verdict. It is meant to be
the plumbing layer. Friendlier human porcelain can sit on top later.

The JSONL stream now separates:

- baseline selection: which prior snapshot was picked
- comparison assessment: how trustworthy that comparison is (`strong`, `usable`, `weak`, or `none`)
- semantic delta: what actually changed
- next action: what the agent should do now

## Documents

- [Flight Recorder Brief](./flight-recorder-brief.md)
  Sponsor users, jobs, hills, non-goals, semantic object model, and output strategy.
- [Playbacks](./playbacks.md)
  Concrete scenarios that define whether the recorder reduces confusion or just adds artifacts.

## Current Product Stance

- The current `pr-snapshot` command is the black box recorder.
- The next slice should only be built if it advances one of the documented hills.
- Raw snapshot-to-snapshot diffing is not enough on its own.
- "What changed?" only matters when the answer is tied to a human decision.
