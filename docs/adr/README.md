<!-- SPDX-License-Identifier: Apache-2.0 OR LicenseRef-MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->

# Architecture Decision Records

Architecture Decision Records preserve decisions whose consequences outlive a
single issue or pull request. They explain why a boundary exists; they do not
track work, progress, priority, or release readiness.

## Contract

- Use a four-digit sequence and a short noun phrase:
  `0002-example-boundary.md`.
- Give every new record a status: `Proposed`, `Accepted`, `Superseded`, or
  `Rejected`.
- Do not rewrite an accepted decision to conceal history. Add a new record and
  mark the old record superseded.
- Record alternatives and consequences, not an implementation diary.
- Keep work state in GitHub issues and pull requests.

## Index

| ADR                                                              | Status      | Decision                                                |
| ---------------------------------------------------------------- | ----------- | ------------------------------------------------------- |
| [0001](ADR-0001-warp-two-plane-skeleton-and-attachments.md)      | Accepted    | Two-plane WARP representation                           |
| [0002](ADR-0002-warp-instances-descended-attachments.md)         | Accepted    | WARP instances and descended attachments                |
| [0003](ADR-0003-Materialization-Bus.md)                          | Implemented | Causality-first ingress and materialization             |
| [0004](ADR-0004-No-Global-State.md)                              | Accepted    | Dependency injection without global state               |
| [0005](ADR-0005-Physics.md)                                      | Accepted    | Deterministic scheduled physics rewrites                |
| [0006](ADR-0006-Ban-Non-Determinism.md)                          | Undeclared  | Ban semantic non-determinism                            |
| [0007](ADR-0007-BOAW-Storage.md)                                 | Accepted    | BOAW storage, execution, merge, and privacy             |
| [0008](ADR-0008-Worldline-Runtime-Model.md)                      | Accepted    | Worldline runtime model                                 |
| [0009](ADR-0009-Inter-Worldline-Communication.md)                | Accepted    | Inter-worldline communication                           |
| [0010](ADR-0010-observational-seek-and-administrative-rewind.md) | Accepted    | Observational seek and administrative rewind            |
| [0011](ADR-0011-explicit-observation-contract.md)                | Implemented | Explicit observation contract                           |
| [0012](0012-repository-knowledge-model.md)                       | Accepted    | Repository knowledge model after Method                 |
| [0013](0013-echo-continuum-authority-boundary.md)                | Accepted    | Echo and Continuum authority boundary                   |
| [0014](0014-generated-rule-authorship-and-footprints.md)         | Accepted    | Generated rule authorship and footprint honesty         |
| [0015](0015-registry-provider-host-boundary.md)                  | Accepted    | Compiler, registry, provider, and host responsibilities |
| [0016](0016-continuum-transport-identity.md)                     | Accepted    | Causal transport identity and idempotence               |
| [0017](0017-universal-little-endian-codec.md)                    | Accepted    | Canonical little-endian binary boundary                 |
| [0018](0018-sessions-causal-posture-and-authority.md)            | Accepted    | Sessions as causal contexts                             |
| [0019](0019-bunny-owns-reusable-geometry.md)                     | Accepted    | Bunny owns reusable geometry                            |
| [0020](0020-retained-reading-storage-and-proof-boundary.md)      | Accepted    | Retained reading storage and proof boundary             |

ADR 0006 predates this index contract and did not declare a status. The index
preserves that fact instead of silently ratifying a historical decision.
