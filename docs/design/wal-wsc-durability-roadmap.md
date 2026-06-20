<!-- SPDX-License-Identifier: Apache-2.0 OR LicenseRef-MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->

# WAL/WSC Durability Doctrine

This document is not the live roadmap. It records the stable WAL/WSC durability
relationship that Echo 1.0 work must preserve. Live planning, issue hierarchy,
dependencies, slice status, and proof state belong in the Echo 1.0 Convergence
Project:

<https://github.com/users/flyingrobots/projects/14>

The owning release issue is
[#521 WAL/WSC Storage Relationship](https://github.com/flyingrobots/echo/issues/521),
which is a child of
[#591 Gate B - Networked Causal Suffix Exchange](https://github.com/flyingrobots/echo/issues/591).

## Doctrine

WAL bytes are the durable commit authority.

WARP graph WAL nodes are projected evidence facts.

WSC carries or references that evidence.

Recovery bootstraps from WAL root or storage manifest material, not from
pre-existing graph facts.

CAS stores bytes by content hash. A content hash is byte identity, not causal
identity, semantic identity, or authorization. Retained evidence references
bind semantic coordinates to byte identity, byte length, role, and artifact
posture.

Storage locators may help recover bytes, but locators are not causal identity.
Causal identity comes from writer identity, epoch, LSN range, segment digest,
commit digest chain, retention references, witnesses, and validated commit
anchors.

## Recovery Model

Echo recovery starts from a configured WAL root or storage manifest. Recovery
validates committed WAL segments, rejects corrupt or incomplete records,
reconstructs indexes and graph projections, then re-emits or verifies retained
evidence references needed by readings, receipts, suffixes, and witnesses.

The graph may contain WAL evidence facts after recovery, but those facts do not
make the WAL recoverable. They are readings over durable history, not the
substrate that creates durable history.

WSC export may be ref-only, self-contained, or CAS-addressed. Each mode must
preserve the same causal identity and the same deterministic admission result:

- Ref-only WSC carries graph facts and stable evidence references.
- Self-contained WSC bundles the bytes required to validate the suffix.
- CAS-addressed WSC names required bytes by content hash and retained evidence
  coordinates.

## GitHub Ownership

The Project uses Goalpost as a cross-repository field. Echo repositories use a
single `Echo 1.0` milestone for release work. Per-goalpost milestones,
spreadsheet-style progress rows, and duplicate repository fields are not part
of the repo contract.

Issue dependencies use GitHub native `blocked by` and `blocking`
relationships. Work slices use native parent/sub-issue relationships under
[#521](https://github.com/flyingrobots/echo/issues/521) and the release gates.

The repository records doctrine and evidence requirements. GitHub records live
work state.

## GP1 Evidence Requirements

The durability substrate goalpost passes only when:

- Defined crash-point matrix passes.
- Recovery is deterministic.
- Retained evidence survives restart.
- Duplicate replay is idempotent.
- Corrupt or incomplete evidence is deterministically rejected.
- Required recovery artifacts are emitted by CI.

Those requirements are release evidence requirements, not prose goals. The
owning issues must link the exact commands, workflow runs, artifacts, and commit
identities that satisfy them.

## Non-Goals

This file must not contain a live task list, current issue inventory, current
status table, branch status, open-count audit, or progress checklist.

This file must not promote graph-projected WAL evidence into the recovery
authority.

This file must not imply that WSC transport arrival is semantic Echo history.
Echo admission is the semantic act.

This file must not imply that retained evidence exists because a content hash
exists. Retention is semantic-coordinate evidence over stored bytes.
