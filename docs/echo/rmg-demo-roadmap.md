# RMG Demo Roadmap (Phase 1 Targets)

This document captures the interactive demos and performance milestones we want to hit as we implement the Rust-based RMG runtime. Each demo proves a key property of Echo’s deterministic multiverse architecture.

---

## Demo 1: Deterministic Netcode

**Goal:** Show two instances running locally in lockstep and prove graph hash equality every frame.

- Two Echo instances (no network) consume identical input streams.
- Each frame emits a “state hash” (BLAKE3 snapshot hash) displayed side-by-side.
- Validation: graph hashes match every tick, proving deterministic state.
- Feature: “frame hash” becomes a first-class inspector metric.

