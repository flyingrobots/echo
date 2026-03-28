<!-- SPDX-License-Identifier: Apache-2.0 OR LicenseRef-MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->

# Flight Recorder Brief

- **Status:** Design brief
- **Date:** 2026-03-25
- **Working name:** Doghouse Flight Recorder
- **Lineage:** Echo-local proving ground for a likely Draft Punks successor

## Problem Statement

PR review state becomes hard to reason about across pushes.

The author sees:

- comments that may be historical or still live
- checks that have rerun, been superseded, or changed state
- a new head SHA with unclear effect on the blocker set
- a GitHub page that encourages rereading instead of reconstruction

The result is state drift, wasted cycles, and low-confidence next actions.

## Sponsor Users

### Primary sponsor user

The PR author inside a noisy multi-round review loop.

They need to understand what changed, what is still blocking merge, and what to do next
without rereading the full PR thread every time.

### Secondary sponsor user

The repo maintainer deciding whether the PR is actually merge-ready.

They need a trustworthy current state summary that separates live blockers from historical
noise.

### Tertiary sponsor user

The coding agent resuming an interrupted PR workflow.

They need a local artifact that reconstructs the current review situation without depending
on memory or terminal scrollback.

## Jobs To Be Done

- When review state becomes confusing across pushes, help the author reconstruct what changed.
- When merge readiness is uncertain, show the current blocker set clearly.
- When a session is interrupted, provide a durable local recovery artifact.

## Hills

### Hill 1: Restore situational awareness

After any push or review round, the author can answer in under 60 seconds:

- what changed since the last meaningful state
- what is blocking merge now
- what action should happen next

### Hill 2: Separate historical noise from live danger

The author can distinguish:

- newly opened unresolved threads
- still-open old threads
- newly resolved threads
- superseded failures
- newly introduced failures

### Hill 3: Preserve durable evidence

An interrupted human or agent can recover:

- current head SHA
- current blocker set
- current unresolved thread set
- current check state
- recent state trajectory

without trusting memory or the GitHub UI alone.

## Non-Goals

- Not a generic GitHub analytics suite.
- Not a replacement for the full PR web page.
- Not yet a response worksheet / adjudication system.
- Not yet organization-wide reporting across repositories.
- Not yet a complete Draft Punks replacement.

## Product Principles

- Trustworthy artifacts beat clever live dashboards.
- Semantic deltas matter more than raw file diffs.
- Local durability matters because GitHub is not a memory system.
- The recorder should reduce mental load, not add another page of bookkeeping.
- Flavor is welcome, but only after the mechanic is trustworthy.

## Core Concepts

### Snapshot

A point-in-time capture of PR state, written locally as JSON + Markdown.

### Sortie

A meaningful review episode:

- a push
- a new automated review round
- a merge-readiness check
- a fix batch resolution pass

### Delta

A semantic comparison between two snapshots that answers "what changed that implies action?"

### Blocker

A merge-relevant condition, such as:

- unresolved review threads
- failing checks
- pending checks
- review decision not approved
- merge state not clean

### Thread transition

A change in unresolved review thread state:

- opened
- resolved
- still open
- reopened, if detectable

### Check transition

A change in check state that affects decision-making:

- fail -> pass
- pending -> pass
- fail -> pending
- pass -> fail
- newly introduced check
- disappeared or superseded check

## What Makes A Delta Meaningful

The recorder should not diff raw JSON and pretend that is insight.

The meaningful delta categories are:

- head transition
- blocker transition
- thread transition
- check transition
- review-state transition
- merge-readiness transition

The recorder should ignore, by default:

- reordered arrays
- timestamp churn
- identical blocker lists with different artifact names
- unchanged thread previews
- raw JSON field differences that do not imply action

## Output Surfaces

### Current slice

- timestamped JSON snapshot
- timestamped Markdown snapshot
- `latest.json`
- `latest.md`

### Intended next surface

A semantic delta report that answers:

- what got better
- what got worse
- what persisted
- what the likely next action is

### Later surface

A sortie timeline or playback view that shows review-state progression without requiring the
author to reconstruct it manually.

## Success Measures

- Fewer manual recounts of unresolved threads.
- Faster resume after interruption.
- Fewer "wait, what changed?" loops in the PR process.
- Fewer unnecessary re-requests for review.
- Faster merge-readiness decisions for noisy PRs.

## Risks

- Building raw snapshot diffing that nobody trusts.
- Overfitting to Echo's workflow and missing the more general product.
- Turning the recorder into a decorative log instead of a decision tool.
- Shipping flavor before the mechanic is sound.

## Relationship To Draft Punks

Draft Punks already identified the core pain: GitHub review state becomes too noisy and too
large to manage directly.

The Flight Recorder is the likely structural successor:

- Draft Punks as the conductor's score
- Flight Recorder as the black box recorder

If the recorder proves itself in Echo, Draft Punks should inherit the mechanic without losing
the voice.

## Immediate Design Decision

The next recorder slice should be designed against the playbacks in
[playbacks.md](./playbacks.md), not against a generic desire to "diff snapshots."
