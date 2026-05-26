<!-- SPDX-License-Identifier: Apache-2.0 OR LicenseRef-MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->

# BEARING Completed Items Archive

Last updated: 2026-05-26.

This archive preserves completed work that previously lived inline in
`docs/BEARING.md`. BEARING should stay focused on the active direction. Closed
slice plans and completed batch notes belong here, in design packets, retros, or
backlog cards.

## Archived From BEARING On 2026-05-26

The following blocks were removed from the active BEARING page because their
checklists were complete:

- Causal WAL slices 1-45: doctrine, grammar, in-memory WAL foundation,
  recovery foundation, submission durability, tick transactions, retention,
  checkpoints, filesystem/object-store posture, outbox, projection, and jedit
  gate doctrine.
- WAL hardening slices 46-65: fixture surface, golden corpus, submission ACK
  crash matrix, tick crash matrix, segment corruption matrix, writer epoch
  matrix, semantic validator negative cases, checkpoint crash matrix, retained
  material matrix, side-effect outbox matrix, reducer determinism, shadow
  replay, causal commit projection, doctor/inspector contract tests,
  crashpoint runner contract, strict sync evidence, object-store negative
  matrix, security/redaction matrix, and hardening release gate.
- WAL segment layout and manifest slices 66-85: canonical segment namespace,
  wall-clock placement guard, legacy flat compatibility, gap/rewrite behavior,
  segment id rotation guard, manifest entry shape, layout release gate,
  manifest-addressed placement doctrine, canonical migration witness, drift
  gate, active segment enforcement, canonical rotation, rotation tail safety,
  multi-segment recovery, rotation authority guard, manifest roundtrip,
  segment-count validation, commit-anchor validation, manifest tail safety, and
  manifest validation release gate.
- Runtime WAL ACK integration slices 86-95: runtime WAL adapter port,
  submission acceptance wiring, duplicate-submit ACK posture, pre-ACK failure
  rollback, tick receipt transaction wiring, commit-before-publish rollback
  guard, runtime index rebuild contract, WAL-backed recovery certificate,
  generic recovery posture contract, and runtime ACK drift gate.
- Recently completed release-gate batch: contract-aware receipts/readings,
  bounded reading identity, contract artifact retention, semantic lookup seams,
  external contract proof fixture, versioned contract/API compatibility,
  reference trusted runtime host loop, serious external consumer fixture, local
  replay/DIND proof, quickstart, and authority audit.

## Canonical Detail Sources

- WAL doctrine and recovery design:
  [`causal-wal-end-to-end.md`](causal-wal-end-to-end.md)
- WAL hardening plan and slice detail:
  [`causal-wal-hardening-matrix.md`](causal-wal-hardening-matrix.md)
- Retained-evidence release-gate batch:
  [`v0.1.0-jedit-next-ten-slices.md`](v0.1.0-jedit-next-ten-slices.md)
- Current work item sequencing:
  [`work-item-sequencing-and-prioritization.md`](work-item-sequencing-and-prioritization.md)

## Completed Outcome Summary

The archived slices establish that Echo has a serious WAL substrate and a
contract-host proof path:

- WAL records and transactions use explicit commit boundaries.
- WAL segment layout is canonical and logical, not wall-clock authoritative.
- WAL recovery can inspect filesystem roots and rebuild generic recovery
  posture.
- Runtime ACK witnesses exist for submission acceptance and scheduler receipt
  publication.
- Contract-host receipts and readings carry package and compatibility evidence.
- QueryView readings carry bounded reading identity and retained evidence
  posture.
- The local contract-host quickstart and authority audit are documented.

## Remaining Active Goal

The next active goal is no longer "prove a WAL exists." The next active goal is
to prove that a sibling application can treat Echo as the durable causal
authority across process death.

That active plan lives in
[`echo-authoritative-jedit-recovery-40-slice-plan.md`](echo-authoritative-jedit-recovery-40-slice-plan.md).
