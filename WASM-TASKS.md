<!-- SPDX-License-Identifier: Apache-2.0 OR MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->

# WASM Task Checklist

Policy: write failing tests first, then implement; check off tasks only when tests and docs are updated.

## P0 — Bootstrap & Scaffold
- [x] Scaffold `specs/spec-000-rewrite` Leptos+Trunk app (CSR) with `index.html`, `src/lib.rs`, panic hook, hot-reload.
- [x] Add workspace membership and `make spec-000-{dev,build}` helpers.
- [ ] Failing check: `cargo check -p spec-000-rewrite --target wasm32-unknown-unknown` in CI job (Trunk build).

## P1 — Kernel Bindings & Types
- [x] Add `wasm-bindgen` feature to kernel crate (or shim crate) and expose minimal RMG/rewrite API (add node, set field, connect, tombstone, materialize).
- [x] Create shared DTO crate (`echo-wasm-abi`) with serde + wasm-bindgen-friendly types for graph and rewrite log; reuse in UI.
- [ ] Failing tests: wasm-bindgen unit test exercising add/set/connect/tombstone round-trip serialization.

## P1 — UI MVP (Living Spec)
- [ ] Render graph (SVG/canvas) from serialized RMG; simple layout.
- [ ] Render rewrite log; click-to-time-travel replays history via kernel API.
- [ ] “Apply Rewrite” panel hooks to kernel methods; updates view reactively.
- [ ] Failing tests: screenshot/DOM snapshot via Playwright (Trunk serve) or headless wasm-bindgen tests for state transitions.

## P2 — Certification & Win Condition
- [ ] Implement completion detector that issues a completion hash/badge when the user reaches target state (Spec-000).
- [ ] Persist/emit completion hash for PR inclusion; document the flow.
- [ ] Failing test: deterministic hash for canonical walkthrough sequence.

## P2 — Tooling & CI
- [ ] GitHub Action: build spec-000 with Trunk (wasm32-unknown-unknown), cache target/Trunk, artifact the dist.
- [ ] Size guard: assert wasm bundle < configured budget; fail if exceeded.
- [ ] Lint: add `cargo fmt`/`clippy` (wasm target) gate for spec crates.

## P3 — UX & Resilience
- [ ] Error surface: UI shows kernel errors (invalid rewrite, payload too large).
- [ ] Offline-first: bundle assets, graceful fallback when no network.
- [ ] Performance pass: incremental graph diffing instead of full redraw; fast layout for ≤200 nodes.
- [ ] Accessibility: keyboard navigation for rewrites; ARIA on controls.

## P3 — Future Spec Template
- [ ] Turn spec-000 into a `spec-template/` scaffold script for future specs (copy, rename, wire to new kernel module, add win condition).
