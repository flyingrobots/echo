<!-- SPDX-License-Identifier: Apache-2.0 OR LicenseRef-MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->

# Playbacks

This document defines the situations the Doghouse Flight Recorder must handle well.

If a future slice does not improve one of these playbacks, it is probably the wrong slice.

## Playback 1: "What changed since my push?"

### Situation: After a push

The author pushes a fix batch and checks back 20 minutes later.

### Current pain: After a push

- new CI runs exist, but the old failed runs are still visible
- some review threads are resolved, some are new, some are historical noise
- the author cannot immediately tell whether the PR improved

### Recorder success condition: After a push

The recorder can tell the author:

- old head -> new head
- blockers removed
- blockers added
- threads newly opened
- threads newly resolved
- checks that improved
- checks that regressed

## Playback 2: "Are we actually ready to merge?"

### Situation: Merge-readiness check

The PR feels done, but GitHub still says blocked.

### Current pain: Merge-readiness check

- some blockers are formal state only
- some blockers are real unresolved work
- the author or maintainer has to reconstruct the answer manually

### Recorder success condition: Merge-readiness check

The recorder can separate:

- live merge blockers
- resolved historical noise
- formal approval-state blockers
- pending automation blockers

## Playback 3: "I got interrupted. What state was I in?"

### Situation: Interrupted resume

An agent or human leaves mid-review cycle and comes back later.

### Current pain: Interrupted resume

- terminal output is gone or noisy
- GitHub comments are too large to reread quickly
- memory is unreliable

### Recorder success condition: Interrupted resume

The latest snapshot plus prior delta can reconstruct:

- current head SHA
- current unresolved thread count
- current check state
- current blocker set
- what changed since the last sortie

## Playback 4: "Did this docs-only push really change anything important?"

### Situation: Docs-only rerun

A small docs or backlog follow-up push restarts the suite and review bots.

### Current pain: Docs-only rerun

- the author knows the push was tiny
- GitHub still creates the impression of a whole new storm
- it is hard to distinguish superficial reruns from substantive new problems

### Recorder success condition: Docs-only rerun

The recorder can show that:

- the head changed
- the blocker set did or did not change
- no new unresolved threads appeared, or exactly which ones did
- failing checks were merely rerun, not substantively regressed

## Playback 5: "Which complaints are actually new?"

### Situation: Historical noise versus new complaints

The PR has been through several rounds and the same themes keep reappearing.

### Current pain: Historical noise versus new complaints

- the author rereads historical comments as if they are current
- GitHub UI makes old major comments feel live
- the review loop burns time on reconstruction

### Recorder success condition: Historical noise versus new complaints

The recorder can distinguish:

- newly opened threads
- old unresolved carry-over threads
- resolved threads that stayed resolved
- reopened or reintroduced issues, if detectable

## Playback 6: "Should this become a Draft Punks capability?"

### Situation: Generalization into Draft Punks

Echo proves the mechanic locally and the team evaluates generalization.

### Current pain: Generalization into Draft Punks

- product ideas drift into tooling ideas
- the original Draft Punks flavor can be lost if the mechanic is too generic

### Recorder success condition: Generalization into Draft Punks

The local artifacts already answer a general problem:

- state reconstruction
- semantic review deltas
- merge-readiness clarity

without depending on Echo-specific assumptions beyond the `gh`-based workflow.

## Anti-Playbacks

These are situations the recorder should not optimize for first:

- generic executive reporting across many repos
- org-wide reviewer scorecards
- full GitHub analytics warehousing
- replacing the PR page entirely
- adjudicating every comment thread as a worksheet system

Those may become relevant later, but they are not the first proof.
