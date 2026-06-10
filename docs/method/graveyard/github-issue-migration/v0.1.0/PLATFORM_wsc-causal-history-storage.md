<!-- SPDX-License-Identifier: Apache-2.0 OR LicenseRef-MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->

# WSC Causal-History Storage

Status: v0.1.0 release doctrine blocker.

Depends on:

- [WAL/WSC storage relationship](./PLATFORM_wal-wsc-storage-relationship.md)
- [Contract retention and semantic lookup seams](./PLATFORM_contract-retention-and-semantic-lookup-seams.md)
- [v0.1.0 replay and DIND proof](./TEST_v0.1.0-replay-dind-proof.md)

## Why now

The jedit release gate is not only "can the app submit and observe once."
Editing history must eventually survive application lifecycles and support
materializing file artifacts from explicit causal points. Echo should define
how WSC participates in that recovery story before implementation choices split
WAL, graph, and export into separate durability systems.

## Required behavior

WSC is the portable container/export boundary for Echo causal-history material.
It may carry:

- WARP graph facts and read-model facts;
- WAL segment evidence refs;
- segment digests and commit anchors;
- recovery certificate refs;
- retained material refs;
- embedded segment bytes or CAS refs when the export is self-contained.

WSC must not become a second recovery authority that can contradict the WAL.

## Export modes

| Mode               | Meaning                                                                                                                                                |
| ------------------ | ------------------------------------------------------------------------------------------------------------------------------------------------------ |
| Ref-only WSC       | Serializes graph facts plus WAL segment locators and digests. Useful when the importer can access the same storage.                                    |
| Self-contained WSC | Serializes graph facts plus embedded WAL segment bytes or bundled retained material. Useful for portable archive and recovery.                         |
| CAS-addressed WSC  | Serializes graph facts plus content-addressed refs to WAL segments and retained material. Useful when CAS storage is shared or transported separately. |

## Acceptance criteria

- [ ] Echo defines WSC export posture for ref-only, self-contained, and
      CAS-addressed causal-history material.
- [ ] Echo defines how exported WSC binds WAL segment digests, LSN ranges, and
      commit anchors.
- [ ] Echo defines how jedit can request materialization from a causal point
      without treating current editor memory as source of truth.
- [ ] Echo defines what happens when an export references unavailable WAL or
      retained material.
- [ ] Echo documents that WSC import validates WAL-backed evidence rather than
      trusting graph facts blindly.

## Test plan

- Add a future WSC export fixture with one committed WAL segment ref and one
  retained material ref.
- Add a future self-contained export fixture proving segment bytes match the
  segment digest and commit chain.
- Add a future obstruction fixture for ref-only WSC whose segment locator is
  unavailable.

## Non-goals

- Do not implement full Continuum replica transport.
- Do not implement social lane or observer-rights governance.
- Do not make jedit file export mutate Echo history.
- Do not make WSC replace the WAL commit boundary.
