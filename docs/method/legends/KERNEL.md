<!-- SPDX-License-Identifier: Apache-2.0 OR LicenseRef-MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->

# KERNEL — Core Simulation Engine

_Legend for the deterministic WARP graph rewrite engine that is Echo's
beating heart._

## Goal

Make Echo's simulation core provably deterministic, honest about its
state transitions, and inspectable at every boundary.

This legend covers work like:

- WARP graph rewrites and the two-plane model
- deterministic scheduling and canonical drain
- tick patches, snapshots, and commit
- parallel execution and canonical merge
- worldline runtime model (heads, seek, replay, fork)
- provenance and causal ordering
- intent ingress and domain separation

## Human users

- James, designing and evolving the simulation kernel
- future contributors extending rewrite rules or scheduling policies
- game/sim developers building on Echo's deterministic guarantees

## Agent users

- agents generating or validating rewrite rules
- agents inspecting simulation state to explain scheduling decisions
- agents writing deterministic tests against kernel behavior

## Human hill

A human can trace any state transition from intent to committed tick
patch and explain why the result is deterministic without reading the
scheduler source.

## Agent hill

An agent can inspect a snapshot, a tick patch, and the provenance log
and programmatically verify that the committed state follows from the
declared rewrite rules and scheduling policy.

## Core invariants

- Deterministic execution: identical inputs produce identical outputs
  on every platform, every run.
- No global state (ADR-0004).
- Two-plane law: skeleton plane is structural, attachment plane is
  data. No hidden edges (ADR-0001).
- Parallel execution produces identical results to serial execution
  (ADR-0007, Phase 6B).
- Observational seek is read-only; administrative rewind is explicit
  (ADR-0010).

## Current cycle and backlog

- latest completed cycle: (none under METHOD yet)
- live backlog:
    - `asap/KERNEL_determinism-torture.md`
    - `asap/KERNEL_domain-separated-hashes.md`
    - `up-next/KERNEL_sha256-blake3.md`
    - `up-next/KERNEL_time-model-spec.md`
