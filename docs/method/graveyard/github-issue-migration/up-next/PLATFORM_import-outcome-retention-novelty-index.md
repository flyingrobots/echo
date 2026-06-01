<!-- SPDX-License-Identifier: Apache-2.0 OR LicenseRef-MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->

# Import outcome retention and novelty index

Status: planned implementation slice.

Depends on:

- [Import transport Intent admission path](../asap/PLATFORM_import-transport-intent-admission-path.md)

## Why now

Once transport import is Intent-driven, repeated import has to be classified by
retained causal evidence instead of folklore. Echo must be able to say whether a
bundle is new, already adjudicated, self-history returning through a peer,
support supplement, alternate support path, or state-equivalent but
witness-distinct.

## Goal

Retain enough import outcome identity to make exact bundle re-import
idempotent and loop posture inspectable.

## Likely files touched

- `crates/warp-core/src/witnessed_suffix.rs`
- `crates/echo-wasm-abi/src/kernel_port.rs`
- `crates/warp-core/src/provenance_store.rs`
- `crates/warp-core/src/coordinator.rs`
- `crates/warp-core/tests/**`
- `docs/design/0022-continuum-transport-identity/design.md`

## Acceptance criteria

- Exact re-import of the same `CausalSuffixBundle` is classified as already
  adjudicated, not fresh admission.
- Self-history arriving through another runtime is classified as loop/self-echo
  posture, not remote novelty.
- Same visible state with different shell or witness identity is not deduped by
  state hash alone.
- Import novelty posture is retained with the local receipt/witness.
- Bundle digest and source shell digest participate in retained import identity.
- Runtime-local ticks, Lamport-like ordering, and local receipt hashes are not
  portable duplicate keys.

## Non-goals

- Do not add network transport.
- Do not require `git-warp`.
- Do not make CAS byte hashes equivalent to causal import identity.
- Do not collapse support supplements into no-op strings.

## Test expectations

- First import records a retained outcome.
- Re-import of the exact same bundle returns deterministic already-adjudicated
  posture.
- Self-echo fixture returns loop/self-history posture.
- State-equivalent but witness-distinct fixture remains distinct.
