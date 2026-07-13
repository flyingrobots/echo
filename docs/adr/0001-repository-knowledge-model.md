<!-- SPDX-License-Identifier: Apache-2.0 OR LicenseRef-MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->

# ADR 0001: Repository Knowledge Model After Method

- **Status:** Accepted
- **Date:** 2026-07-13
- **Decision owner:** James Ross

## Context

Echo's former Method system stored backlog lanes, cycle packets, retrospectives,
status ledgers, dependency graphs, audits, and execution plans in the repository.
Those artifacts grew faster than the implementation, duplicated GitHub state,
and routinely described work or components that no longer existed.

Deleting that system without a replacement would also be a mistake. Echo still
needs durable architectural memory, a clear source-of-truth hierarchy, and an
evidence-first engineering discipline.

## Decision

Repository knowledge has five non-overlapping homes:

1. **GitHub issues, pull requests, and projects own motion.** Priority, current
   work, blockers, review discussion, handoffs, and release coordination live
   there. They are not mirrored into checked-in plans.
2. **`docs/adr/` owns durable decisions.** ADRs record accepted choices,
   alternatives, consequences, and explicit supersession.
3. **`docs/topics/` owns living architectural truth.** A topic describes the
   current boundary and changes when the implementation boundary changes.
4. **Specs and invariants own normative contracts.** `docs/spec/`,
   `docs/invariants/`, and directly relevant architecture documents define what
   conforming behavior means.
5. **Executable witnesses own proof.** Tests, fixtures, schema validators,
   deterministic vectors, and CI gates prove claims. `CHANGELOG.md` records
   behavior that actually shipped.

Echo will not check in backlogs, cycle packets, retrospectives, status reports,
review transcripts, point-in-time audits, or roadmap checklists as substitutes
for those sources.

## Engineering Discipline Retained

Abandoning Method does not abandon rigor:

- Bound every change to a behavioral, structural, or documentation claim.
- Name the smallest executable witness before editing.
- For non-trivial behavior, record the design and test plan in the issue or pull
  request; create an ADR first when the work changes a durable boundary.
- Prefer RED, then the smallest GREEN implementation, then relevant surrounding
  verification.
- Update living topics, specs, invariants, and `CHANGELOG.md` when their truth
  changes.
- Stop when the requested claim is green; file unrelated work in GitHub.

## Consequences

- Git history remains the archive for removed process artifacts.
- A clean checkout no longer attempts to answer "what should happen next?";
  GitHub does.
- A clean checkout must still answer "what is Echo?", "who owns this
  boundary?", "what decision governs it?", and "what proves it?".
- Documentation review becomes architectural review instead of bookkeeping
  review.
