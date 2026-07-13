<!-- SPDX-License-Identifier: Apache-2.0 OR LicenseRef-MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->

# ADR-0004: No Global Runtime State

- **Status:** Accepted
- **Date:** 2026-01-14

## Context

Mutable process-wide state creates hidden initialization order, ambient
authority, cross-test interference, and native/WASM divergence. Even
memory-safe singleton mechanisms can make behavior depend on which installer
ran first rather than on the explicit causal basis and runtime configuration.

## Decision

Echo runtime dependencies are owned by constructed values and passed
explicitly. Registries, codecs, stores, clocks, ingress, schedulers, and other
capabilities live in runtime structures or typed parameters; consumers do not
install them into mutable process-wide singletons.

Protected runtime crates forbid mutable global state, thread-local runtime
authority, lazy singleton installation, and `static mut`. Immutable constants
and static lookup data are allowed when they carry no mutable authority or
initialization-order semantics.

Narrow exceptions must be executable, scoped, and justified beside the live
enforcement policy. An allowlist entry is evidence of a boundary exception; it
does not silently amend this architectural decision.

## Rejected Alternatives

- Install registries or runtime services into process-wide singleton cells.
- Use thread-local state to hide dependencies from APIs.
- Permit test-only global mutation that can leak between cases.
- Duplicate the enforcement script inside this ADR.

## Consequences

- Runtime construction exposes its dependencies and capabilities.
- Tests can build isolated runtimes without shared initialization order.
- Native and WASM adapters use the same explicit ownership model.
- Enforcement evolves in executable scripts and CI without turning the ADR
  into a stale second implementation.

## Evidence Anchors

- `scripts/ban-globals.sh`
- `.ban-globals-allowlist`
- `.github/workflows/ci.yml`
- `crates/warp-core/src/engine_impl.rs`
- `crates/warp-wasm/src/warp_kernel.rs`

## Historical Note

The original record embedded a copy of the ban script, allowlist instructions,
CI wiring, and promotional README text. Those implementation artifacts remain
in Git history; the live script is the executable witness.
