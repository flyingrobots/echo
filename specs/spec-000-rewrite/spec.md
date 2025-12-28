<!-- SPDX-License-Identifier: Apache-2.0 OR MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->
# SPEC-000: Everything Is a Rewrite

**Status:** Draft (live demo scaffold)

## Purpose
Teach the core JITOS principle: all durable state evolves via immutable, semantic, reversible graph rewrites.

## Concepts Demonstrated
1. Immutable append-only rewrites
2. Reversible events (old + new values)
3. SemanticOps (intent-aware changes)
4. Graph materialization (apply rewrites → current view)
5. Time travel (step backward/forward through rewrites)

## Demo Outline
- Graph viewer (nodes/edges/fields)
- Rewrite log (click to replay)
- Apply rewrite panel (add node, set field, connect, tombstone)

## Win Condition (Phase 0 scaffold)
- The Spec-000 page builds and runs in the browser via `make spec-000-dev`.
- The UI renders the embedded Spec-000 markdown and supports a deterministic “epoch” counter increment.
- Next milestone (not yet implemented): wire `echo-wasm-bindings::DemoKernel` into the UI, replay rewrites, and emit a deterministic completion proof.

## Implementation Notes
- UI: Leptos CSR (Trunk)
- Kernel: `echo-wasm-bindings` DemoKernel (to be replaced with real engine bindings)
- ABI: `echo-wasm-abi` shared DTOs
