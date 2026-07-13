<!-- SPDX-License-Identifier: Apache-2.0 OR LicenseRef-MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->

# Architecture Decision Records

Architecture Decision Records preserve decisions whose consequences outlive a
single issue or pull request. They explain why a boundary exists; they do not
track work, progress, priority, or release readiness.

## Contract

- Use a four-digit sequence and a short noun phrase:
  `0002-example-boundary.md`.
- Give every record a status: `Proposed`, `Accepted`, `Superseded`, or
  `Rejected`.
- Do not rewrite an accepted decision to conceal history. Add a new record and
  mark the old record superseded.
- Record alternatives and consequences, not an implementation diary.
- Keep work state in GitHub issues and pull requests.

## Index

| ADR                                                      | Status   | Decision                                                |
| -------------------------------------------------------- | -------- | ------------------------------------------------------- |
| [0001](0001-repository-knowledge-model.md)               | Accepted | Repository knowledge model after Method                 |
| [0002](0002-echo-continuum-authority-boundary.md)        | Accepted | Echo and Continuum authority boundary                   |
| [0003](0003-generated-rule-authorship-and-footprints.md) | Accepted | Generated rule authorship and footprint honesty         |
| [0004](0004-registry-provider-host-boundary.md)          | Accepted | Compiler, registry, provider, and host responsibilities |
| [0005](0005-continuum-transport-identity.md)             | Accepted | Causal transport identity and idempotence               |
| [0006](0006-universal-little-endian-codec.md)            | Accepted | Canonical little-endian binary boundary                 |
| [0007](0007-sessions-causal-posture-and-authority.md)    | Accepted | Sessions as causal contexts                             |
| [0008](0008-bunny-owns-reusable-geometry.md)             | Accepted | Bunny owns reusable geometry                            |
