<!-- SPDX-License-Identifier: Apache-2.0 OR LicenseRef-MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->

# ADR 0005: Continuum Transport Identity

- **Status:** Accepted
- **Date:** 2026-07-13

## Context

Retries, reordered delivery, storage relocation, and multiple carriers make
transport-local identity unsuitable for deterministic import and replay.

## Decision

Import identity and idempotence bind to canonical witnessed causal evidence:
source identity, causal basis, suffix or claim identity, payload and witness
digests, and the named admission law. They do not bind to a filename, segment
path, connection, arrival position, wall-clock timestamp, or mutable peer
cursor.

Re-observing the same adjudicated evidence yields stable duplicate posture. A
changed claim is a new proposal or a protocol violation, never a silent
overwrite.

## Consequences

- Carriers may retransmit or relocate bytes without changing semantic identity.
- Arrival order cannot decide scheduler order.
- Recovery and replication can explain duplication from retained evidence.

## Evidence Anchors

- `docs/architecture/continuum-transport.md`
- `crates/warp-core/src/witnessed_suffix.rs`
- `crates/warp-core/src/admission.rs`
