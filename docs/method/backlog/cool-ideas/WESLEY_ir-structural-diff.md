<!-- SPDX-License-Identifier: Apache-2.0 OR LicenseRef-MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->

# `wesley diff <old-ir> <new-ir>`: structural classifier for schema migrations

Legend: `PLATFORM`

## The motivation

The 2026-05-30 review pass on PR #382 surfaced a class of bug that
was structurally hard to spot from a code diff alone: stale
`codec_id` in inline IR fixtures, enum reorders that would silently
break `SCHEMA_SHA256` parity with mixed-version peers, append-only
enum-compatibility claims that the framing itself contradicted.

These are not code review smells — they are _schema migration
smells_. They all share a shape: "the IR changed in a way that
looks small but reaches outside the file via hash preimages, op-id
derivation, or cross-language wire contracts."

A `wesley diff` tool would classify these structurally instead of
relying on a reviewer's ability to remember which IR edits are
load-bearing.

## What it classifies

Given two Wesley IR JSON files (or their generated artifacts), emit
a structured report:

**Breaking (will fail mixed-version peers):**

- enum reorder
- enum append (because `SCHEMA_SHA256` is the version gate;
  appending changes the hash and breaks any peer not on the new
  schema — the design.md for cycle 0024 carries this exact claim
  now after the doc-fix in PR #382)
- field removal
- field type change
- field cardinality change (single → list, nullable → required,
  etc.)
- `codec_id` change
- op-id preimage change (operation rename, kind change,
  reordering that affects FNV step inputs)

**Potentially safe (still version-gated, but additive):**

- new operation
- new type not referenced by any existing operation or field
- additive field with explicit default (if/when the IR supports
  one)

**Drift (smells but doesn't break):**

- inline-IR `codec_id` mismatch with `DEFAULT_CODEC_ID`
- comment / documentation changes
- whitespace / ordering in non-significant positions

## Output shape

JSON (machine-readable, for CI gating) and a human-readable
summary (for code review). Example:

```text
$ wesley diff old.ir.json new.ir.json

Breaking changes (1):
  - Enum reorder: TextDirection variant order changed from
    [Ltr, Rtl] to [Rtl, Ltr]. SCHEMA_SHA256 will change.

Potentially safe additions (2):
  - New operation: createCheckpoint
  - New type: CheckpointKind (referenced by createCheckpoint)

Drift (1):
  - Inline IR codec_id "cbor-canon-v1" differs from
    DEFAULT_CODEC_ID "le-binary-v1"
```

## Where it lives

The natural home is the Wesley repo itself (alongside the emitter
and the IR validator). Echo could vendor a thin wrapper for CI.
The card belongs in echo because that is the repo that just paid
the cost of not having it; the actual implementation will need a
Wesley cycle.

## How it gets used

- Pre-push gate on any change that touches a `.ir.json` or a
  `.graphql` schema: run `wesley diff origin/main HEAD` and refuse
  the push if breaking changes are present without a version bump.
- PR review: a bot posts the diff classification on every schema-
  touching PR. Reviewers no longer need to mentally model "did
  this hash change?"
- Local: developers can ask `wesley diff` before committing to
  preview what their change implies.

## Why this prevents the most review churn

- The 0024-design append-only claim (corrected in PR #382) was a
  doc bug rooted in a misread of the version gate. `wesley diff`
  would have flagged the proposed enum append as "breaking
  (SCHEMA_SHA256 change)" the moment the IR moved, eliminating the
  doc inconsistency at the source.
- Inline-IR `codec_id` drift across the eight `tests/generation.rs`
  fixtures would have been surfaced as "drift" the first time it
  diverged from `DEFAULT_CODEC_ID`, instead of accumulating across
  cycles.
- The current `op_id` derivation duplication between echo and
  wesley-core (carded in
  `bad-code/PLATFORM_echo-wesley-gen-local-emitter-duplication.md`)
  becomes safer to collapse because the diff tool can prove that
  the upstream output matches the local output across the entire
  pinned vector set.

## Out of scope here

- Visualizing the diff in a UI. JSON + text is enough for the
  90% case.
- Auto-fixing breaking changes (e.g. proposing a version bump).
  Classification is a complete unit of work; mitigation is a
  follow-up.

## Companion

- `docs/method/backlog/bad-code/PLATFORM_echo-wesley-gen-local-emitter-duplication.md`
  — partially blocked by the existence of this tool (the upstream
  collapse becomes safer with a diff classifier).
- `docs/method/backlog/cool-ideas/PLATFORM_wesley-emitted-fixture-vectors.md`
  — adjacent: cross-boundary fixture parity. Wesley diff classifies
  IR changes; emitted fixture vectors classify wire-byte changes.
  Both close the same family of bugs from different angles.
