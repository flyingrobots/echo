<!-- SPDX-License-Identifier: Apache-2.0 OR LicenseRef-MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->

# ADR 0007: Sessions as Causal Posture and Authority

- **Status:** Accepted
- **Date:** 2026-07-13

## Context

The word "session" previously mixed transport connections, viewer state,
editing context, privacy, and runtime authority. Those concepts do not share an
identity or lifecycle.

## Decision

A session is a causal context: a bounded relationship among an authority
domain, causal basis, participants, revelation posture, and retained evidence.
It is not a socket, mutable server object, or permission flag.

`Scratch`, `AuthorOnly`, and `Shared` name causal and revelation postures.
Promotion to a wider posture is a new witnessed append-only act; it does not
mutate past privacy. Authority binds to an explicit authority-domain reference
and remains distinct from transport presence.

## Consequences

- Disconnecting does not erase admitted history or transfer authority.
- UI presence and transport membership cannot stand in for causal membership.
- Sharing requires explicit evidence and can be obstructed under policy.

## Evidence Anchors

- `crates/warp-core/src/revelation.rs`
- `crates/warp-core/src/strand.rs`
- `crates/warp-core/src/sealed_membership.rs`
- `docs/topics/RuntimeAuthority.md`
