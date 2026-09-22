<!-- SPDX-License-Identifier: Apache-2.0 OR LicenseRef-MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->

# Legacy Architecture Decision Archive

These numbered records preserve Echo decisions accepted before the repository
adopted concept-owned documentation. They remain useful historical evidence,
but their numbers are locators only: sequence does not express dependency,
supersession, refinement, importance, or current authority.

The archive closed after record 0026 on 2026-08-09. New durable decisions update
the semantically named topic, architecture document, specification, invariant,
or colocated rationale that owns the concept. The
[documentation standards](../DOCUMENTATION_STANDARDS.md) define that current
contract.

## Archive contract

- Do not allocate another numbered record.
- Preserve accepted historical text; add a clear status or relationship note
  when current documentation supersedes or refines it.
- Record supersession in both the historical record and its new semantic owner.
- Follow explicit descriptive links; never infer a relationship from numbers.
- Keep work state in GitHub issues and pull requests.

Some older records predate the relationship contract and name only a status.
When one is materially revisited, add descriptive forward and reverse links
without rewriting its original reasoning.

## Index

| ADR                                                              | Status               | Decision                                                       |
| ---------------------------------------------------------------- | -------------------- | -------------------------------------------------------------- |
| [0001](ADR-0001-warp-two-plane-skeleton-and-attachments.md)      | Accepted             | Two-plane WARP representation                                  |
| [0002](ADR-0002-warp-instances-descended-attachments.md)         | Accepted             | WARP instances and descended attachments                       |
| [0003](ADR-0003-Materialization-Bus.md)                          | Superseded           | Historical causality-first ingress and materialization         |
| [0004](ADR-0004-No-Global-State.md)                              | Accepted             | Dependency injection without global state                      |
| [0005](ADR-0005-Physics.md)                                      | Superseded           | Historical deterministic physics proposal                      |
| [0006](ADR-0006-Ban-Non-Determinism.md)                          | Superseded           | Historical non-determinism enforcement proposal                |
| [0007](ADR-0007-BOAW-Storage.md)                                 | Partially superseded | BOAW execution mechanics; ADR 0020 governs storage authority   |
| [0008](ADR-0008-Worldline-Runtime-Model.md)                      | Accepted             | Worldline runtime model                                        |
| [0009](ADR-0009-Inter-Worldline-Communication.md)                | Superseded           | Historical frontier-relative state-patch transport             |
| [0010](ADR-0010-observational-seek-and-administrative-rewind.md) | Accepted             | Observational seek and administrative rewind                   |
| [0011](ADR-0011-explicit-observation-contract.md)                | Partially superseded | Explicit observation mechanics; ADR 0021 governs public optics |
| [0012](0012-repository-knowledge-model.md)                       | Superseded           | Historical repository knowledge model after Method             |
| [0013](0013-echo-continuum-authority-boundary.md)                | Accepted             | Echo and Continuum authority boundary                          |
| [0014](0014-generated-rule-authorship-and-footprints.md)         | Accepted             | Generated rule authorship and footprint honesty                |
| [0015](0015-registry-provider-host-boundary.md)                  | Partially superseded | Compiler, registry, provider, and host responsibilities        |
| [0016](0016-continuum-transport-identity.md)                     | Accepted             | Causal transport identity and idempotence                      |
| [0017](0017-universal-little-endian-codec.md)                    | Accepted             | Canonical little-endian binary boundary                        |
| [0018](0018-sessions-causal-posture-and-authority.md)            | Accepted             | Sessions as causal contexts                                    |
| [0019](0019-bunny-owns-reusable-geometry.md)                     | Accepted             | Bunny owns reusable geometry                                   |
| [0020](0020-retained-reading-storage-and-proof-boundary.md)      | Accepted             | Retained reading storage and proof boundary                    |
| [0021](0021-public-optic-observation-boundary.md)                | Accepted             | Public WARP optic over internal observation                    |
| [0022](0022-application-requested-causal-anchor-admission.md)    | Accepted             | Application-requested, Echo-owned anchor admission             |
| [0023](0023-admitted-executable-operation-packages.md)           | Accepted             | Admitted executable operation packages                         |
| [0024](0024-anchored-node-creation-from-absence.md)              | Accepted             | Anchored-node creation as a separate executable program        |
| [0025](0025-scheduler-owned-executable-operation-actions.md)     | Accepted             | Scheduler-owned executable-operation Actions                   |
| [0026](0026-durable-external-action-settlement.md)               | Accepted             | Request-before-effect and settlement-before-resumption         |

ADR 0006 predates this index contract and did not declare a status. Its
superseded tombstone preserves that fact without silently ratifying the old
implementation proposal.
